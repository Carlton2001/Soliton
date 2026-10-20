use crate::{
    catalog::Filter,
    i18n::{Text, text},
    input::TextInput,
    scrollbar::Scrollbar,
    service::{Event, Services, Work},
    theme::Theme,
};
use gpui::{prelude::*, *};
use rap_core::{
    config::{Language, Settings},
    model::LibrarySnapshot,
};
use rap_library::{Cancellation, ScanProgress};
use rap_platform::AppPaths;
use std::{
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
    pub pending_removal: Option<i64>,
    pub settings: Settings,
    pub snapshot: Arc<LibrarySnapshot>,
    pub visible: Vec<usize>,
    pub filter: Filter,
    pub section: Section,
    pub selected: Option<i64>,
    pub search: Entity<TextInput>,
    pub services: Services,
    pub scanning: bool,
    pub cancellation: Cancellation,
    pub progress: ScanProgress,
    pub status: String,
    pub error: Option<String>,
    pub columns: usize,
    pub facets: crate::catalog::Facets,
    pub filter_menu: Option<&'static str>,
    query_pending: bool,
    _subscriptions: Vec<Subscription>,
}

impl Rap {
    pub fn new(settings: Settings, paths: AppPaths, cx: &mut Context<Self>) -> Self {
        let (audio, audio_error) = match rap_audio::Engine::start(settings.volume) {
            Ok(engine) => (Some(engine), None),
            Err(error) => (None, Some(error.to_string())),
        };
        let playback = rap_audio::Playback {
            volume: settings.volume,
            ..Default::default()
        };
        let search = cx.new(TextInput::new);
        let subscription = cx.observe(&search, |this, search, cx| {
            let query = search.read(cx).content.to_string();
            if query != this.filter.query {
                this.filter.query = query;
                this.selected = None;
                this.query_pending = true;
            }
        });
        let lists = [(); 3].map(|_| UniformListScrollHandle::new());
        let bar =
            |list: &UniformListScrollHandle| Scrollbar::new(list.0.borrow().base_handle.clone());
        let mut this = Self {
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
            pending_removal: None,
            settings,
            snapshot: Arc::new(LibrarySnapshot::default()),
            visible: Vec::new(),
            filter: Filter::default(),
            section: Section::Albums,
            selected: None,
            search,
            services: Services::start(paths),
            scanning: false,
            cancellation: Cancellation::default(),
            progress: ScanProgress::default(),
            status: String::new(),
            error: audio_error,
            columns: 4,
            facets: Default::default(),
            filter_menu: None,
            query_pending: true,
            _subscriptions: vec![subscription],
        };
        this.scan(Vec::new(), cx);
        cx.spawn(async move |this, cx| {
            let mut last_refresh = Instant::now();
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(60))
                    .await;
                if this
                    .update(cx, |this, cx| {
                        let mut changed = false;
                        if let Some(audio) = &this.audio {
                            if let Some(playback) = audio.events.try_iter().last() {
                                changed |= playback.state != this.playback.state
                                    || playback.position_ms != this.playback.position_ms
                                    || playback.volume != this.playback.volume
                                    || playback.shuffle != this.playback.shuffle
                                    || playback.repeat != this.playback.repeat
                                    || playback.error != this.playback.error;
                                if playback.error.is_some() && playback.error != this.playback.error
                                {
                                    this.error = playback.error.clone();
                                }
                                this.playback = playback;
                            }
                        }
                        for event in this.services.events.try_iter() {
                            match event {
                                Event::Progress(progress) => this.progress = progress,
                                Event::Finished(cancelled) => {
                                    this.scanning = false;
                                    this.status = this
                                        .t(if cancelled {
                                            Text::Cancelled
                                        } else {
                                            Text::Ready
                                        })
                                        .into();
                                    this.query_pending = true;
                                }
                                Event::Removed => this.query_pending = true,
                                Event::Error(error) => this.error = Some(error),
                            }
                            changed = true;
                        }
                        for result in this.services.views.try_iter() {
                            match result {
                                Ok(view) if view.filter == this.filter => {
                                    this.snapshot = view.snapshot;
                                    this.visible = view.visible;
                                    this.facets = view.facets;
                                    this.keyboard_index = this
                                        .keyboard_index
                                        .min(this.visible.len().saturating_sub(1));
                                }
                                Ok(_) => {}
                                Err(error) => this.error = Some(format!("{error:#}")),
                            }
                            changed = true;
                        }
                        if this.scanning && last_refresh.elapsed() > Duration::from_millis(750) {
                            this.query_pending = true;
                            last_refresh = Instant::now();
                        }
                        if this.query_pending
                            && this.services.query.try_send(this.filter.clone()).is_ok()
                        {
                            this.query_pending = false;
                        }
                        if changed {
                            cx.notify();
                        }
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

    pub fn t(&self, key: Text) -> &'static str {
        text(self.settings.language, key)
    }
    pub fn theme(&self) -> Theme {
        Theme::new(self.settings.dark)
    }
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.query_pending = true;
        cx.notify();
    }
    pub fn save(&mut self) {
        if let Err(error) = self
            .services
            .work
            .try_send(Work::Save(self.settings.clone()))
        {
            self.error = Some(error.to_string());
        }
    }
    pub fn scan(&mut self, paths: Vec<std::path::PathBuf>, cx: &mut Context<Self>) {
        if self.scanning {
            return;
        }
        self.cancellation = Cancellation::default();
        self.progress = ScanProgress::default();
        self.error = None;
        match self
            .services
            .work
            .try_send(Work::Scan(paths, self.cancellation.clone()))
        {
            Ok(()) => self.scanning = true,
            Err(error) => self.error = Some(error.to_string()),
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
                    this.error = Some(format!("{error:?}"));
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
            self.error = Some(error.to_string());
        }
        cx.notify();
    }
    pub fn navigate(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        self.selected = None;
        cx.notify();
    }
    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.filter = Filter::default();
        self.search.update(cx, |input, cx| {
            input.reset();
            cx.notify();
        });
        self.refresh(cx);
    }

    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.control && key == "f" {
            self.search.read(cx).focus_handle(cx).focus(window);
            return;
        }
        if event.keystroke.modifiers.control && key == "o" {
            self.open_file(cx);
            return;
        }
        if key == "escape" {
            self.selected = None;
            self.filter_menu = None;
            self.focus.focus(window);
            cx.notify();
            return;
        }
        if self.search.read(cx).focus_handle(cx).is_focused(window) {
            return;
        }
        if key == "space" {
            self.play_pause(cx);
            return;
        }
        if self.section != Section::Albums || self.visible.is_empty() {
            return;
        }
        let step = if self.settings.album_grid {
            self.columns
        } else {
            1
        };
        self.keyboard_index = match key {
            "left" => self.keyboard_index.saturating_sub(1),
            "right" => (self.keyboard_index + 1).min(self.visible.len() - 1),
            "up" => self.keyboard_index.saturating_sub(step),
            "down" => (self.keyboard_index + step).min(self.visible.len() - 1),
            "enter" => {
                self.selected = Some(
                    self.snapshot.albums
                        [self.visible[self.keyboard_index.min(self.visible.len() - 1)]]
                    .id,
                );
                cx.notify();
                return;
            }
            _ => return,
        };
        self.album_scroll
            .scroll_to_item(self.keyboard_index / step, ScrollStrategy::Center);
        cx.notify();
    }
    pub fn set_language(&mut self, language: Language, cx: &mut Context<Self>) {
        self.settings.language = language;
        self.save();
        cx.notify();
    }
    pub fn set_dark(&mut self, dark: bool, cx: &mut Context<Self>) {
        self.settings.dark = dark;
        self.save();
        cx.notify();
    }
}

impl Drop for Rap {
    fn drop(&mut self) {
        self.cancellation.cancel();
    }
}

impl Render for Rap {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.columns = ((f32::from(window.viewport_size().width) - 260.) / 185.)
            .floor()
            .max(1.) as usize;
        let theme = self.theme();
        let placeholder = self.t(Text::Search);
        self.search
            .update(cx, |input, _| input.placeholder = placeholder.into());
        let body = match self.section {
            Section::Settings => self.settings_view(cx),
            _ if self.selected.is_some() => self.album_view(cx),
            Section::Albums => self.albums_view(cx),
            _ => self.facets_view(cx),
        };
        div()
            .relative()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key))
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .text_color(theme.text)
            .font_family("Segoe UI")
            .text_size(px(14.))
            .child(
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
                    ),
            )
            .when_some(self.error.clone(), |view, error| {
                view.child(
                    div()
                        .px_4()
                        .py_2()
                        .bg(theme.card)
                        .flex()
                        .gap_3()
                        .child(self.t(Text::PlaybackError))
                        .child(div().flex_1().truncate().child(error))
                        .child(crate::widgets::button("dismiss", "x", "", theme).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.error = None;
                                cx.notify();
                            }),
                        )),
                )
            })
            .child(self.footer(cx))
            .when(self.filter_menu.is_some(), |view| {
                view.child(self.filter_menu_view(cx))
            })
    }
}
