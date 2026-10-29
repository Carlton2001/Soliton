use crate::{
    catalog::{Facets, Filter, FilterOptions},
    filters::Menu,
    i18n::{Text, text},
    input::TextInput,
    notices::{Level, Notices},
    scrollbar::Scrollbar,
    service::{Event, Services, Work},
    slider::Slider,
    theme::Theme,
};
use gpui::{prelude::*, *};
use soliton_core::{
    config::{Accent, AlbumSort, Appearance, CoverSize, Language, Settings},
    failure::{Failure, Kind},
    model::{Album, LibrarySnapshot},
};
use soliton_library::{Cancellation, ScanProgress};
use soliton_platform::AppPaths;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Albums,
    Artists,
    Playlists,
    Queue,
    Settings,
}

/// Library tracks by id.
pub type Tracks = Arc<std::collections::HashMap<i64, soliton_core::model::Track>>;

pub struct Soliton {
    pub updates: crate::updates::Updates,
    pub now_playing: bool,
    pub volume_popup: bool,
    pub tracks: Tracks,
    /// Queue and catalog last compared for track availability.
    pub availability_checked: Option<(Arc<soliton_audio::QueueSnapshot>, Tracks)>,
    pub track_menu: Option<(crate::track_list::Selection, bool)>,
    pub playlists: crate::playlists::Playlists,
    pub queue_scroll: UniformListScrollHandle,
    pub queue_bar: Scrollbar,
    pub desktop: Option<soliton_platform::Desktop>,
    pub autostart: Option<bool>,
    pub startup_pending: bool,
    pub bluetooth: crate::bluetooth_preferences::BluetoothState,
    pub output_watch: crate::output_watch::OutputWatch,
    pub history: crate::history::History,
    pub resume: crate::resume::Resume,
    pub confirm_close: bool,
    pub quitting: bool,
    pub audio: Option<soliton_audio::Engine>,
    pub playback: soliton_audio::Playback,
    pub focus: FocusHandle,
    pub modal_focus: FocusHandle,
    pub keyboard_index: usize,
    pub album_scroll: UniformListScrollHandle,
    pub album_bar: Scrollbar,
    pub facet_scroll: UniformListScrollHandle,
    pub facet_bar: Scrollbar,
    pub track_scroll: UniformListScrollHandle,
    pub track_bar: Scrollbar,
    pub settings_bar: Scrollbar,
    pub menu_scroll: UniformListScrollHandle,
    pub menu_bar: Scrollbar,
    pub pending_removal: Option<i64>,
    pub settings: Settings,
    /// Operating system preference, used when the appearance follows the system.
    pub system_dark: bool,
    pub system_accent: Option<[u32; 2]>,
    pub snapshot: Arc<LibrarySnapshot>,
    pub visible: Vec<usize>,
    pub filter: Filter,
    pub section: Section,
    /// Page the search text was typed on.
    pub(super) search_section: Section,
    pub selected: Option<i64>,
    /// Index into the selected album's editions.
    pub selected_edition: usize,
    pub search: Entity<TextInput>,
    /// Narrows the values listed in the filter menu.
    pub menu_search: Entity<TextInput>,
    pub services: Services,
    pub scanning: bool,
    pub cancellation: Cancellation,
    pub progress: ScanProgress,
    pub notices: Notices,
    pub columns: usize,
    pub facets: Arc<Facets>,
    pub filter_options: Arc<FilterOptions>,
    pub menu: Option<Menu>,
    pub seek_slider: Slider,
    pub volume_slider: Slider,
    /// Position shown while the seek slider is dragged; sent on release.
    pub seek_preview: Option<f32>,
    /// Volume restored when sound is turned back on.
    pub muted_volume: Option<f32>,
    /// Last seek sent and when, shown until the engine reports it.
    pub seek_target: Option<(u64, Instant)>,
    query_pending: bool,
    _subscriptions: Vec<Subscription>,
}

impl Soliton {
    pub fn new(
        settings: Settings,
        paths: AppPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (audio, audio_error) =
            match soliton_audio::Engine::start(settings.volume, settings.output_device.clone()) {
                Ok(engine) => (Some(engine), None),
                Err(error) => (None, Some(error.to_string())),
            };
        // Sent before the saved queue is loaded, so that it keeps its order.
        if let Some(engine) = &audio {
            let _ = engine.send(soliton_audio::Command::Shuffle(settings.shuffle));
            let _ = engine.send(soliton_audio::Command::Repeat(settings.repeat));
        }
        let playback = soliton_audio::Playback {
            volume: settings.volume,
            shuffle: settings.shuffle,
            repeat: settings.repeat,
            selected_output: settings.output_device.clone(),
            ..Default::default()
        };
        let search = cx.new(TextInput::new);
        let menu_search = cx.new(TextInput::new);
        let playlist_name = cx.new(TextInput::new);
        let subscriptions = vec![
            // Kept in memory and written with the other settings at exit.
            cx.observe_window_bounds(window, |this, window, _| {
                if let Some(placement) = crate::placement::capture(window) {
                    this.settings.window = Some(placement);
                }
            }),
            // The field searches albums on the album pages and filters the
            // list shown elsewhere, which only needs a new render.
            cx.observe(&search, |this, search, cx| {
                let query = if this.section == Section::Albums {
                    search.read(cx).content.to_string()
                } else {
                    String::new()
                };
                if query != this.filter.query {
                    if !query.is_empty() {
                        this.selected = None;
                    }
                    this.filter.query = query;
                    this.query_pending = true;
                } else if this.section != Section::Albums {
                    cx.notify();
                }
            }),
            cx.observe(&menu_search, |_, _, cx| cx.notify()),
            cx.observe(&playlist_name, |_, _, cx| cx.notify()),
            cx.observe_window_appearance(window, |this, window, cx| {
                this.system_dark = is_dark(window.appearance());
                cx.notify();
            }),
        ];
        let lists = [(); 4].map(|_| UniformListScrollHandle::new());
        let bar =
            |list: &UniformListScrollHandle| Scrollbar::new(list.0.borrow().base_handle.clone());
        let mut last = settings.last_view.clone();
        // Compatibility with settings written before the Home page was removed.
        if last.view == soliton_core::config::View::Home {
            last.view = soliton_core::config::View::Artists;
            last.album = None;
        }
        // Settings are not a place to come back to: reopen on the queue when
        // one is waiting, otherwise on the artists.
        if last.view == soliton_core::config::View::Settings {
            last.view = if settings.last_queue.tracks.is_empty() {
                soliton_core::config::View::Artists
            } else {
                soliton_core::config::View::Queue
            };
        }
        let filter = Filter {
            sort: settings.album_sort,
            hide_unavailable: settings.hide_unavailable,
            artist: last.artist.clone(),
            ..Default::default()
        };
        let mut this = Self {
            updates: Default::default(),
            now_playing: false,
            volume_popup: false,
            tracks: Arc::default(),
            availability_checked: None,
            track_menu: None,
            playlists: crate::playlists::Playlists::new(playlist_name),
            queue_scroll: UniformListScrollHandle::new(),
            queue_bar: Scrollbar::default(),
            desktop: None,
            autostart: None,
            startup_pending: false,
            bluetooth: Default::default(),
            output_watch: Default::default(),
            history: Default::default(),
            resume: crate::resume::Resume::new(),
            confirm_close: false,
            quitting: false,
            audio,
            playback,
            focus: cx.focus_handle(),
            modal_focus: cx.focus_handle(),
            keyboard_index: 0,
            album_bar: bar(&lists[0]),
            album_scroll: lists[0].clone(),
            facet_scroll: lists[1].clone(),
            track_scroll: lists[2].clone(),
            facet_bar: bar(&lists[1]),
            track_bar: bar(&lists[2]),
            settings_bar: Scrollbar::default(),
            menu_scroll: lists[3].clone(),
            menu_bar: bar(&lists[3]),
            pending_removal: None,
            settings,
            system_dark: is_dark(window.appearance()),
            system_accent: None,
            snapshot: Arc::new(LibrarySnapshot::default()),
            visible: Vec::new(),
            filter,
            section: Section::from(last.view),
            search_section: Section::from(last.view),
            selected: last.album,
            selected_edition: 0,
            search,
            menu_search,
            services: Services::start(paths),
            scanning: false,
            cancellation: Cancellation::default(),
            progress: ScanProgress::default(),
            notices: Notices::default(),
            columns: 4,
            facets: Default::default(),
            filter_options: Default::default(),
            menu: None,
            seek_slider: Slider::default(),
            volume_slider: Slider::default(),
            seek_preview: None,
            muted_volume: None,
            seek_target: None,
            query_pending: true,
            _subscriptions: subscriptions,
        };
        if let Some(error) = audio_error {
            let text = this.labelled(this.t(Text::AudioUnavailable), &error);
            this.report_error(text);
        }
        this.playlists.selected = last.playlist;
        this.queue_bar = bar(&this.queue_scroll);
        this.start_desktop(window);
        this.scan(Vec::new(), cx);
        if this.settings.check_updates {
            this.check_for_updates(cx);
        }
        cx.spawn_in(window, async move |this, cx| {
            let mut last_refresh = Instant::now();
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(60))
                    .await;
                if this
                    .update_in(cx, |this, window, cx| {
                        this.poll(&mut last_refresh, cx);
                        this.poll_desktop(window, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        this
    }

    /// Applies engine states, service events and query results.
    fn poll(&mut self, last_refresh: &mut Instant, cx: &mut Context<Self>) {
        self.poll_bluetooth(cx);
        self.poll_outputs(cx);
        let mut changed = false;
        if let Some(playback) = self.audio.as_ref().and_then(|a| a.events.try_iter().last()) {
            if playback.queue.current != self.playback.queue.current
                && let Some(index) = playback
                    .queue
                    .entries
                    .iter()
                    .position(|e| Some(e.id) == playback.queue.current)
            {
                self.queue_scroll
                    .scroll_to_item(index, ScrollStrategy::Center);
            }
            changed |= playback.state != self.playback.state
                || !Arc::ptr_eq(&playback.queue, &self.playback.queue)
                || playback.position_ms != self.playback.position_ms
                || playback.volume != self.playback.volume
                || playback.shuffle != self.playback.shuffle
                || playback.repeat != self.playback.repeat
                || playback.error != self.playback.error
                || playback.output != self.playback.output
                || playback.selected_output != self.playback.selected_output
                || playback.outputs != self.playback.outputs
                || playback.output_error != self.playback.output_error
                || playback.fallback != self.playback.fallback
                || playback.interruption != self.playback.interruption
                || playback.output_changing != self.playback.output_changing
                || playback.outputs_error != self.playback.outputs_error
                || playback.network_outputs != self.playback.network_outputs
                || playback.network_output != self.playback.network_output
                || playback.network_discovering != self.playback.network_discovering
                || playback.network_enabled != self.playback.network_enabled
                || playback.network_error != self.playback.network_error
                || playback.warning != self.playback.warning
                || playback.track.as_ref().map(|t| t.id)
                    != self.playback.track.as_ref().map(|t| t.id);
            if let Some(error) = &playback.error
                && playback.error != self.playback.error
            {
                self.report_failure(Level::Error, Text::Playback, error);
            }
            if let Some(warning) = &playback.warning
                && playback.warning != self.playback.warning
            {
                self.report_failure(Level::Warning, Text::Playback, warning);
            }
            let output_errors: Vec<_> = [
                (
                    &playback.output_error,
                    &self.playback.output_error,
                    Text::AudioOutput,
                ),
                (
                    &playback.outputs_error,
                    &self.playback.outputs_error,
                    Text::AudioOutput,
                ),
                (
                    &playback.network_error,
                    &self.playback.network_error,
                    Text::NetworkOutput,
                ),
            ]
            .into_iter()
            .filter_map(|(error, previous, label)| {
                error
                    .as_ref()
                    .filter(|_| error != previous)
                    .map(|error| (label, error.clone()))
            })
            .collect();
            for (label, error) in output_errors {
                self.report_failure(Level::Error, label, &error);
            }
            // Save only an acknowledged choice; rejected changes keep the old one.
            if playback.selected_output != self.settings.output_device {
                self.settings.output_device = playback.selected_output.clone();
                self.save();
            }
            if playback.network_output.is_some() {
                self.settings.volume = playback.volume;
            }
            let previous_track = self.playback.track.as_ref().map(|t| t.id);
            self.playback = playback;
            self.settle_seek(previous_track);
            self.remember_queue();
        }
        for event in self.services.events.try_iter().collect::<Vec<_>>() {
            match event {
                Event::Playlists(lists) => {
                    self.playlists.lists = lists;
                    self.playlists.pending = false;
                    if self
                        .playlists
                        .selected
                        .is_some_and(|id| !self.playlists.lists.iter().any(|p| p.id == id))
                    {
                        self.playlists.selected = None;
                    }
                }
                Event::Progress(progress) => self.progress = progress,
                Event::Finished(cancelled) => {
                    self.scanning = false;
                    if cancelled {
                        self.report_warning(self.t(Text::Cancelled));
                    }
                    self.query_pending = true;
                }
                Event::Removed | Event::Changed => self.query_pending = true,
                Event::SourceUnavailable(path) => {
                    let detail = if self.settings.hide_unavailable {
                        Text::SourceUnavailableHidden
                    } else {
                        Text::SourceUnavailable
                    };
                    let text = self.labelled(&path.display().to_string(), self.t(detail));
                    self.report_warning(text);
                    self.query_pending = true;
                }
                Event::Warning(failure) => {
                    self.report_failure(Level::Warning, service_label(&failure), &failure)
                }
                Event::Error(failure) => {
                    self.report_failure(Level::Error, service_label(&failure), &failure)
                }
            }
            changed = true;
        }
        for result in self.services.views.try_iter().collect::<Vec<_>>() {
            match result {
                Ok(view) if view.filter == self.filter => {
                    self.tracks = view.tracks;
                    self.snapshot = view.snapshot;
                    self.restore_queue(cx);
                    // A restored or open album may have left the library.
                    if self
                        .selected
                        .is_some_and(|id| !self.snapshot.albums.iter().any(|a| a.id == id))
                    {
                        self.selected = None;
                    }
                    self.visible = view.visible;
                    self.facets = view.facets;
                    self.filter_options = view.options;
                    self.keyboard_index = self
                        .keyboard_index
                        .min(self.visible.len().saturating_sub(1));
                }
                Ok(_) => {}
                Err(error) => {
                    self.report_failure(Level::Error, Text::Library, &Failure::of(&error))
                }
            }
            changed = true;
        }
        if self.scanning && last_refresh.elapsed() > Duration::from_millis(750) {
            self.query_pending = true;
            *last_refresh = Instant::now();
        }
        self.sync_queue_availability();
        if self.query_pending && self.services.query.try_send(self.filter.clone()).is_ok() {
            self.query_pending = false;
        }
        if changed {
            cx.notify();
        }
    }

    pub fn t(&self, key: Text) -> &'static str {
        text(self.settings.language, key)
    }
    pub fn labelled(&self, label: &str, value: &str) -> String {
        crate::i18n::labelled(self.settings.language, label, value)
    }
    pub fn dark(&self) -> bool {
        match self.settings.appearance {
            Appearance::System => self.system_dark,
            Appearance::Light => false,
            Appearance::Dark => true,
        }
    }
    pub fn theme(&self) -> Theme {
        Theme::with_system_accent(self.dark(), self.settings.accent, self.system_accent)
    }
    /// Queries the catalogue again with the current filter and display settings.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.filter.sort = self.settings.album_sort;
        self.filter.hide_unavailable = self.settings.hide_unavailable;
        self.query_pending = true;
        cx.notify();
    }
    pub fn save(&mut self) {
        self.services.settings.save(self.settings.clone());
    }
    pub fn scan(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        if self.scanning {
            return;
        }
        self.cancellation = Cancellation::default();
        self.progress = ScanProgress::default();
        match self
            .services
            .work
            .try_send(Work::Scan(paths, self.cancellation.clone()))
        {
            Ok(()) => self.scanning = true,
            Err(_) => self.report_failure(Level::Error, Text::Library, &Failure::new(Kind::Busy)),
        }
        cx.notify();
    }
    pub fn choose_source(&mut self, cx: &mut Context<Self>) {
        if self.scanning {
            return;
        }
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some(self.t(Text::AddSource).into()),
        });
        cx.spawn(async move |this, cx| match prompt.await {
            Ok(Ok(Some(paths))) => {
                let paths = cx
                    .background_executor()
                    .spawn(
                        async move { paths.into_iter().map(soliton_platform::host_path).collect() },
                    )
                    .await;
                let _ = this.update(cx, |this, cx| this.scan(paths, cx));
            }
            Ok(Ok(None)) => {}
            error => {
                let _ = this.update(cx, |this, cx| {
                    this.report_error(format!("{error:?}"));
                    cx.notify();
                });
            }
        })
        .detach();
    }
    /// Removal only forgets catalogue entries; it waits for any running scan.
    pub fn remove_source(&mut self, id: i64, cx: &mut Context<Self>) {
        self.pending_removal = None;
        if self.scanning {
            return;
        }
        if self.services.work.try_send(Work::RemoveSource(id)).is_err() {
            self.report_failure(Level::Error, Text::Library, &Failure::new(Kind::Busy));
        }
        cx.notify();
    }
    pub fn navigate(&mut self, section: Section, cx: &mut Context<Self>) {
        self.record_place(cx);
        self.volume_popup = false;
        self.now_playing = false;
        self.section = section;
        self.keyboard_index = 0;
        self.selected = None;
        self.track_menu = None;
        self.menu = None;
        cx.notify();
    }
    /// Shows an album sheet on its best edition.
    pub fn open_album(&mut self, id: i64, cx: &mut Context<Self>) {
        self.record_place(cx);
        self.volume_popup = false;
        self.track_menu = None;
        self.menu = None;
        self.now_playing = false;
        self.selected = Some(id);
        self.selected_edition = 0;
        if self.section != Section::Albums {
            self.section = Section::Albums;
        }
        self.track_scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }
    /// Lists the albums of an artist.
    /// Albums of `artist`; an empty name opens the albums without any artist.
    pub fn show_artist(&mut self, artist: String, cx: &mut Context<Self>) {
        self.record_place(cx);
        self.now_playing = false;
        self.menu = None;
        self.track_menu = None;
        self.filter.artist = Some(artist);
        self.section = Section::Albums;
        self.selected = None;
        self.refresh(cx);
    }
    /// Displayed album artist, with translated labels for compilations and
    /// albums without any artist.
    pub fn album_artist(&self, album: &Album) -> String {
        if album.artist.is_empty() && album.compilation {
            self.t(Text::VariousArtists).into()
        } else {
            self.artist_label(&album.artist)
        }
    }
    pub fn artist_label(&self, artist: &str) -> String {
        if artist.is_empty() {
            self.t(Text::UnknownArtist).into()
        } else {
            artist.to_owned()
        }
    }
    pub fn album_label(&self, album: &str) -> String {
        if album.is_empty() {
            self.t(Text::UnknownAlbum).into()
        } else {
            album.to_owned()
        }
    }
    /// `Artist · Album` under a track title.
    pub fn track_subtitle(&self, metadata: &soliton_core::model::Metadata) -> String {
        format!(
            "{} · {}",
            self.artist_label(&metadata.artist),
            self.album_label(&metadata.album)
        )
    }
    /// Removes search-independent filters; display settings are kept.
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.filter = Filter::default();
        self.search.update(cx, |input, cx| {
            input.reset();
            cx.notify();
        });
        self.refresh(cx);
    }
    /// Leaves the innermost view: menu, now playing, album sheet.
    pub fn back(&mut self, cx: &mut Context<Self>) {
        if self.now_playing && self.menu.is_none() && self.track_menu.is_none() {
            self.volume_popup = false;
            self.now_playing = false;
            cx.notify();
            return;
        }
        if self.playlists.dialog.take().is_some() || self.track_menu.take().is_some() {
            cx.notify();
            return;
        }
        if self.section == Section::Playlists && self.playlists.selected.take().is_some() {
            cx.notify();
            return;
        }
        if self.menu.take().is_none() && self.selected.is_some() {
            self.return_from_album(cx);
        }
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.settings.language = language;
        self.save();
        cx.notify();
    }
    pub fn set_appearance(&mut self, appearance: Appearance, cx: &mut Context<Self>) {
        self.settings.appearance = appearance;
        self.save();
        cx.notify();
    }
    pub fn set_accent(&mut self, accent: Accent, cx: &mut Context<Self>) {
        self.settings.accent = accent;
        self.save();
        cx.notify();
    }
    pub fn set_ui_rem_size(&mut self, size: u32, cx: &mut Context<Self>) {
        let size = soliton_core::config::Settings::clamp_rem_size(size);
        if size != self.settings.ui_rem_size {
            self.settings.ui_rem_size = size;
            self.save();
            cx.notify();
        }
    }
    pub fn set_cover_size(&mut self, size: CoverSize, cx: &mut Context<Self>) {
        self.settings.cover_size = size;
        self.save();
        cx.notify();
    }
    /// Grid or list for the page shown: albums and artists keep their own choice.
    pub fn set_grid(&mut self, grid: bool, cx: &mut Context<Self>) {
        if self.section == Section::Artists {
            self.settings.artist_grid = grid;
        } else {
            self.settings.album_grid = grid;
        }
        self.save();
        cx.notify();
    }
    pub fn set_sort(&mut self, sort: AlbumSort, cx: &mut Context<Self>) {
        self.settings.album_sort = sort;
        self.save();
        self.album_scroll.scroll_to_item(0, ScrollStrategy::Top);
        self.refresh(cx);
    }
    pub fn set_hide_unavailable(&mut self, hide: bool, cx: &mut Context<Self>) {
        self.settings.hide_unavailable = hide;
        self.save();
        self.refresh(cx);
    }
}

fn is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

impl Drop for Soliton {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

impl Render for Soliton {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::scale::apply(self.settings.ui_rem_size, window);
        // Tile and gap widths, then the list padding and scrollbar margin.
        let gap = crate::scale::scaled(crate::views::TILE_GAP * 16.);
        // Cover, padding, then the 2 px focus border on each side.
        let tile =
            crate::scale::scaled(crate::views::cover_side(self.settings.cover_size) + 16.) + 4.;
        // The artist page also keeps a lane for its letter index.
        let lane = if self.section == Section::Artists {
            crate::alphabet::LANE
        } else {
            0.
        };
        let reserved =
            crate::scale::scaled((2. * crate::views::CONTENT_PADDING + lane) * 16. + 12.);
        let width_in_rems = crate::navigation::content_width(window);
        let split = crate::collection::split_collection(
            width_in_rems,
            self.section,
            self.selected.is_some(),
        );
        let panel_width = if split {
            crate::scale::scaled(crate::collection::album_panel_width(width_in_rems) * 16.)
        } else {
            0.
        };
        // Only the collection header may drop labels: see `header_density`.
        crate::widgets::set_density(crate::widgets::Density::Full);
        let content = width_in_rems * f32::from(window.rem_size());
        let available = content - panel_width - reserved;
        self.columns = ((available + gap) / (tile + gap)).floor().max(1.) as usize;
        let theme = self.theme();
        // Each page has its own search: the text does not follow to another one.
        let section = self.section;
        let changed = std::mem::replace(&mut self.search_section, section) != section;
        let placeholder = self.t(match section {
            Section::Artists => Text::SearchArtists,
            _ => Text::Search,
        });
        self.search.update(cx, |input, cx| {
            input.placeholder = placeholder.into();
            if changed && !input.content.is_empty() {
                input.reset();
                cx.notify();
            }
        });
        self.record_place(cx);
        let find = self.t(Text::Find);
        self.menu_search
            .update(cx, |input, _| input.placeholder = find.into());
        if f32::from(window.viewport_size().width) / f32::from(window.rem_size()) >= 44. {
            self.volume_popup = false;
        }
        let body = if self.now_playing {
            self.now_playing_view(window, cx)
        } else {
            match self.section {
                Section::Queue => self.queue_view(cx),
                Section::Playlists => self.playlists_view(cx),
                Section::Settings => self.settings_view(cx),
                Section::Albums => self.collection_view(window, cx),
                _ if self.selected.is_some() => self.album_view(window, cx),
                Section::Artists => self.artists_view(cx),
            }
        };
        let main = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .overflow_hidden()
            .when(
                !self.now_playing && self.section == Section::Artists && self.selected.is_none(),
                |v| {
                    v.child(self.page_title(Text::Artists, cx))
                        .child(self.measured_header(window, cx))
                },
            )
            .child(body);
        let notices = self.notices_view(cx);
        let fullscreen = window.is_fullscreen();
        let content = div()
            .relative()
            .track_focus(&self.focus)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    if this.volume_popup {
                        this.volume_popup = false;
                        cx.notify();
                    }
                }),
            )
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Back),
                cx.listener(|this, _, _, cx| this.go_back(cx)),
            )
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Forward),
                cx.listener(|this, _, _, cx| this.go_forward(cx)),
            )
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.text)
            .font_family(crate::widgets::UI_FONT)
            .text_size(crate::widgets::TEXT_BODY)
            // Full screen is a listening mode: the library chrome stays hidden.
            .when(fullscreen, |content| {
                content.child(self.fullscreen_view(window, cx))
            })
            .when(!fullscreen, |content| {
                content.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .flex()
                        .child(self.navigation(window, cx))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .min_h_0()
                                .flex()
                                .flex_col()
                                .child(self.title_bar(window, cx))
                                .child(main),
                        ),
                )
            })
            .children(notices)
            .when(!fullscreen, |content| {
                content
                    .child(self.footer(window, cx))
                    .children(self.menu_view(cx))
            })
            .children(self.close_dialog(cx))
            .when(!fullscreen, |content| {
                content.children(self.playlist_dialog(cx))
            });
        crate::window_chrome::frame(content, window, theme)
    }
}

/// Library workers also save settings; their failures say which.
fn service_label(failure: &Failure) -> Text {
    match failure.kind {
        Kind::SettingsNotSaved => Text::Settings,
        _ => Text::Library,
    }
}
