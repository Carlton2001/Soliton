use crate::{
    catalog::{Facets, Filter},
    filters::Menu,
    i18n::{Text, text},
    input::TextInput,
    notices::Notices,
    scrollbar::Scrollbar,
    service::{Event, Services, Work},
    slider::Slider,
    theme::Theme,
};
use gpui::{prelude::*, *};
use rap_core::{
    config::{Accent, AlbumSort, Appearance, CoverSize, Language, Settings},
    model::{Album, LibrarySnapshot},
};
use rap_library::{Cancellation, ScanProgress};
use rap_platform::AppPaths;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Albums,
    Artists,
    Genres,
    Years,
    Settings,
}

pub struct Rap {
    pub desktop: Option<rap_platform::Desktop>,
    pub autostart: Option<bool>,
    pub startup_pending: bool,
    pub confirm_close: bool,
    pub quitting: bool,
    pub audio: Option<rap_audio::Engine>,
    pub playback: rap_audio::Playback,
    pub focus: FocusHandle,
    pub keyboard_index: usize,
    pub album_scroll: UniformListScrollHandle,
    pub album_bar: Scrollbar,
    pub facet_scroll: UniformListScrollHandle,
    pub facet_bar: Scrollbar,
    pub track_scroll: UniformListScrollHandle,
    pub track_bar: Scrollbar,
    pub settings_bar: Scrollbar,
    pub menu_scroll: UniformListScrollHandle,
    pub pending_removal: Option<i64>,
    pub settings: Settings,
    /// Operating system preference, used when the appearance follows the system.
    pub system_dark: bool,
    pub snapshot: Arc<LibrarySnapshot>,
    pub visible: Vec<usize>,
    pub filter: Filter,
    pub section: Section,
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
    pub menu: Option<Menu>,
    pub now_playing: bool,
    pub seek_slider: Slider,
    pub volume_slider: Slider,
    /// Position shown while the seek slider is dragged; sent on release.
    pub seek_preview: Option<f32>,
    /// Volume restored when sound is turned back on.
    pub muted_volume: Option<f32>,
    /// Files dropped or opened for one-off playback are being read.
    pub preparing: Option<Cancellation>,
    pub loose_covers: PathBuf,
    query_pending: bool,
    _subscriptions: Vec<Subscription>,
}

impl Rap {
    pub fn new(
        settings: Settings,
        paths: AppPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (audio, audio_error) = match rap_audio::Engine::start(settings.volume) {
            Ok(engine) => (Some(engine), None),
            Err(error) => (None, Some(error.to_string())),
        };
        let playback = rap_audio::Playback {
            volume: settings.volume,
            ..Default::default()
        };
        let search = cx.new(TextInput::new);
        let menu_search = cx.new(TextInput::new);
        let subscriptions = vec![
            cx.observe(&search, |this, search, cx| {
                let query = search.read(cx).content.to_string();
                if query != this.filter.query {
                    this.filter.query = query;
                    this.selected = None;
                    if this.section == Section::Settings {
                        this.section = Section::Albums;
                    }
                    this.query_pending = true;
                }
            }),
            cx.observe(&menu_search, |_, _, cx| cx.notify()),
            cx.observe_window_appearance(window, |this, window, cx| {
                this.system_dark = is_dark(window.appearance());
                cx.notify();
            }),
        ];
        let lists = [(); 3].map(|_| UniformListScrollHandle::new());
        let bar =
            |list: &UniformListScrollHandle| Scrollbar::new(list.0.borrow().base_handle.clone());
        let filter = Filter {
            sort: settings.album_sort,
            hide_unavailable: settings.hide_unavailable,
            ..Default::default()
        };
        let mut this = Self {
            desktop: None,
            autostart: None,
            startup_pending: false,
            confirm_close: false,
            quitting: false,
            audio,
            playback,
            focus: cx.focus_handle(),
            keyboard_index: 0,
            album_bar: bar(&lists[0]),
            album_scroll: lists[0].clone(),
            facet_scroll: lists[1].clone(),
            track_scroll: lists[2].clone(),
            facet_bar: bar(&lists[1]),
            track_bar: bar(&lists[2]),
            settings_bar: Scrollbar::default(),
            menu_scroll: UniformListScrollHandle::new(),
            pending_removal: None,
            settings,
            system_dark: is_dark(window.appearance()),
            snapshot: Arc::new(LibrarySnapshot::default()),
            visible: Vec::new(),
            filter,
            section: Section::Albums,
            selected: None,
            selected_edition: 0,
            search,
            menu_search,
            loose_covers: paths.covers.join("loose"),
            services: Services::start(paths),
            scanning: false,
            cancellation: Cancellation::default(),
            progress: ScanProgress::default(),
            notices: Notices::default(),
            columns: 4,
            facets: Default::default(),
            menu: None,
            now_playing: false,
            seek_slider: Slider::default(),
            volume_slider: Slider::default(),
            seek_preview: None,
            muted_volume: None,
            preparing: None,
            query_pending: true,
            _subscriptions: subscriptions,
        };
        if let Some(error) = audio_error {
            let text = this.labelled(this.t(Text::AudioUnavailable), &error);
            this.report_error(text);
        }
        this.start_desktop(window);
        this.scan(Vec::new(), cx);
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
        let mut changed = false;
        if let Some(playback) = self.audio.as_ref().and_then(|a| a.events.try_iter().last()) {
            changed |= playback.state != self.playback.state
                || playback.position_ms != self.playback.position_ms
                || playback.volume != self.playback.volume
                || playback.shuffle != self.playback.shuffle
                || playback.repeat != self.playback.repeat
                || playback.error != self.playback.error
                || playback.track.as_ref().map(|t| t.id)
                    != self.playback.track.as_ref().map(|t| t.id);
            if let Some(error) = &playback.error
                && playback.error != self.playback.error
            {
                self.report_error(error.clone());
            }
            self.playback = playback;
        }
        for event in self.services.events.try_iter().collect::<Vec<_>>() {
            match event {
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
                    let text =
                        self.labelled(&path.display().to_string(), self.t(Text::SourceUnavailable));
                    self.report_warning(text);
                    self.query_pending = true;
                }
                Event::Warning(text) => self.report_warning(text),
                Event::Error(text) => self.report_error(text),
            }
            changed = true;
        }
        for result in self.services.views.try_iter().collect::<Vec<_>>() {
            match result {
                Ok(view) if view.filter == self.filter => {
                    self.snapshot = view.snapshot;
                    self.visible = view.visible;
                    self.facets = view.facets;
                    self.keyboard_index = self
                        .keyboard_index
                        .min(self.visible.len().saturating_sub(1));
                }
                Ok(_) => {}
                Err(error) => self.report_error(format!("{error:#}")),
            }
            changed = true;
        }
        if self.scanning && last_refresh.elapsed() > Duration::from_millis(750) {
            self.query_pending = true;
            *last_refresh = Instant::now();
        }
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
        Theme::new(self.dark(), self.settings.accent)
    }
    /// Queries the catalogue again with the current filter and display settings.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.filter.sort = self.settings.album_sort;
        self.filter.hide_unavailable = self.settings.hide_unavailable;
        self.query_pending = true;
        cx.notify();
    }
    pub fn save(&mut self) {
        if let Err(error) = self
            .services
            .work
            .try_send(Work::Save(self.settings.clone()))
        {
            self.report_error(error.to_string());
        }
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
            Err(error) => self.report_error(error.to_string()),
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
        if let Err(error) = self.services.work.try_send(Work::RemoveSource(id)) {
            self.report_error(error.to_string());
        }
        cx.notify();
    }
    pub fn navigate(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        self.selected = None;
        self.now_playing = false;
        self.menu = None;
        cx.notify();
    }
    /// Shows an album sheet on its best edition.
    pub fn open_album(&mut self, id: i64, cx: &mut Context<Self>) {
        self.selected = Some(id);
        self.selected_edition = 0;
        self.now_playing = false;
        if self.section == Section::Settings {
            self.section = Section::Albums;
        }
        self.track_scroll.scroll_to_item(0, ScrollStrategy::Top);
        cx.notify();
    }
    /// Lists the albums of an artist.
    pub fn show_artist(&mut self, artist: String, cx: &mut Context<Self>) {
        if artist.is_empty() {
            return;
        }
        self.filter.artist = Some(artist);
        self.section = Section::Albums;
        self.selected = None;
        self.now_playing = false;
        self.refresh(cx);
    }
    /// Displayed album artist, with a translated label for compilations.
    pub fn album_artist(&self, album: &Album) -> String {
        if album.artist.is_empty() && album.compilation {
            self.t(Text::VariousArtists).into()
        } else {
            album.artist.clone()
        }
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
        if self.menu.take().is_none() && !std::mem::take(&mut self.now_playing) {
            self.selected = None;
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
    pub fn set_cover_size(&mut self, size: CoverSize, cx: &mut Context<Self>) {
        self.settings.cover_size = size;
        self.save();
        cx.notify();
    }
    pub fn set_grid(&mut self, grid: bool, cx: &mut Context<Self>) {
        self.settings.album_grid = grid;
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

impl Drop for Rap {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Some(preparing) = &self.preparing {
            preparing.cancel();
        }
    }
}

impl Render for Rap {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tile = crate::views::cover_side(self.settings.cover_size) + 32.;
        self.columns = ((f32::from(window.viewport_size().width) - 260.) / tile)
            .floor()
            .max(1.) as usize;
        let theme = self.theme();
        let placeholder = self.t(Text::Search);
        self.search
            .update(cx, |input, _| input.placeholder = placeholder.into());
        let find = self.t(Text::Find);
        self.menu_search
            .update(cx, |input, _| input.placeholder = find.into());
        let main = if self.now_playing {
            self.now_playing_view(window, cx)
        } else {
            let body = match self.section {
                Section::Settings => self.settings_view(cx),
                _ if self.selected.is_some() => self.album_view(cx),
                Section::Albums => self.albums_view(cx),
                _ => self.facets_view(cx),
            };
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(self.sidebar(cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .child(self.header(cx))
                        .child(body),
                )
                .into_any_element()
        };
        let notices = self.notices_view(cx);
        div()
            .relative()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key))
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Back),
                cx.listener(|this, _, _, cx| this.back(cx)),
            )
            .on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| {
                this.play_paths(paths.paths().to_vec(), cx)
            }))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.text)
            .font_family("Segoe UI")
            .text_size(px(14.))
            .child(main)
            .children(notices)
            .when(!self.now_playing, |view| view.child(self.footer(cx)))
            .children(self.menu_view(cx))
            .child(self.drop_overlay())
            .children(self.close_dialog(cx))
    }
}
