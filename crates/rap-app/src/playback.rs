use crate::{
    app::Rap,
    i18n::Text,
    widgets::{button, cover, icon_button, time},
};
use gpui::{prelude::*, *};
use rap_audio::{Command, PlaybackState, Repeat};

impl Rap {
    pub fn audio_command(&mut self, command: Command, cx: &mut Context<Self>) {
        if let Some(audio) = &self.audio {
            if let Err(error) = audio.send(command) {
                self.error = Some(error.to_string());
            }
        }
        cx.notify();
    }
    pub fn play_album(&mut self, id: i64, start: usize, cx: &mut Context<Self>) {
        if let Some(album) = self.snapshot.albums.iter().find(|a| a.id == id) {
            if album.tracks.get(start).is_none_or(|t| !t.available) {
                return;
            }
            let start_id = album.tracks[start].id;
            let tracks: Vec<_> = album
                .tracks
                .iter()
                .filter(|t| t.available)
                .cloned()
                .collect();
            let start = tracks.iter().position(|t| t.id == start_id).unwrap_or(0);
            self.audio_command(Command::Play { tracks, start }, cx);
        }
    }
    pub fn play_pause(&mut self, cx: &mut Context<Self>) {
        if self.playback.state == PlaybackState::Playing {
            self.audio_command(Command::Pause, cx);
        } else if self.playback.track.is_some() {
            self.audio_command(Command::Resume, cx);
        } else if let Some(id) = self
            .selected
            .or_else(|| self.visible.first().map(|&i| self.snapshot.albums[i].id))
        {
            self.play_album(id, 0, cx);
        }
    }
    pub fn volume(&mut self, value: f32, cx: &mut Context<Self>) {
        self.settings.volume = value.clamp(0., 1.);
        self.save();
        self.audio_command(Command::Volume(self.settings.volume), cx);
    }
    pub fn open_file(&mut self, cx: &mut Context<Self>) {
        let prompt = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some(self.t(Text::OpenFile).into()),
        });
        cx.spawn(async move |this, cx| match prompt.await {
            Ok(Ok(Some(paths))) => {
                let tracks = paths
                    .into_iter()
                    .map(|path| rap_core::model::Track {
                        id: 0,
                        source_id: 0,
                        metadata: rap_core::model::Metadata {
                            title: path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                            ..Default::default()
                        },
                        path,
                        cover: None,
                        available: true,
                    })
                    .collect();
                let _ = this.update(cx, |this, cx| {
                    this.audio_command(Command::Play { tracks, start: 0 }, cx)
                });
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

    pub fn player(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let seek_bounds = std::rc::Rc::new(std::cell::Cell::new(Bounds::<Pixels>::default()));
        let painted_bounds = seek_bounds.clone();
        let theme = self.theme();
        let p = &self.playback;
        let repeat = match p.repeat {
            Repeat::Off => Text::RepeatOff,
            Repeat::Track => Text::RepeatTrack,
            Repeat::All => Text::RepeatAll,
        };
        let next_repeat = match p.repeat {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::Track,
            Repeat::Track => Repeat::Off,
        };
        let progress = if p.duration_ms > 0 {
            p.position_ms as f32 / p.duration_ms as f32
        } else {
            0.
        };
        div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_2()
            .px_4()
            .py_3()
            .bg(theme.panel)
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(cover(
                        p.track.as_ref().and_then(|t| t.cover.as_ref()),
                        44.,
                        theme,
                    ))
                    .child(
                        div()
                            .w(px(200.))
                            .min_w_0()
                            .child(
                                div().truncate().font_weight(FontWeight::SEMIBOLD).child(
                                    p.track
                                        .as_ref()
                                        .map(|t| t.metadata.title.clone())
                                        .unwrap_or_else(|| self.t(Text::NothingPlaying).into()),
                                ),
                            )
                            .child(
                                div().truncate().text_xs().text_color(theme.muted).child(
                                    p.track
                                        .as_ref()
                                        .map(|t| t.metadata.artist.clone())
                                        .unwrap_or_default(),
                                ),
                            ),
                    )
                    .child(
                        icon_button("previous", "skip-back", self.t(Text::Previous), theme)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.audio_command(Command::Previous, cx)
                            })),
                    )
                    .child(
                        button(
                            "play-pause",
                            if p.state == PlaybackState::Playing {
                                "pause"
                            } else {
                                "play"
                            },
                            self.t(if p.state == PlaybackState::Playing {
                                Text::Pause
                            } else {
                                Text::Play
                            }),
                            theme,
                        )
                        .bg(theme.card)
                        .on_click(cx.listener(|this, _, _, cx| this.play_pause(cx))),
                    )
                    .child(
                        icon_button("next", "skip-forward", self.t(Text::Next), theme).on_click(
                            cx.listener(|this, _, _, cx| this.audio_command(Command::Next, cx)),
                        ),
                    )
                    .child(
                        icon_button("stop", "square", self.t(Text::Stop), theme).on_click(
                            cx.listener(|this, _, _, cx| this.audio_command(Command::Stop, cx)),
                        ),
                    )
                    .child(div().flex_1())
                    .child(button("volume-down", "volume-2", "−", theme).on_click(
                        cx.listener(|this, _, _, cx| this.volume(this.settings.volume - 0.05, cx)),
                    ))
                    .child(format!(
                        "{} {}%",
                        self.t(Text::Volume),
                        (self.settings.volume * 100.).round()
                    ))
                    .child(button("volume-up", "volume-2", "+", theme).on_click(
                        cx.listener(|this, _, _, cx| this.volume(this.settings.volume + 0.05, cx)),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        button("seek-back", "skip-back", self.t(Text::SeekBack), theme).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.audio_command(
                                    Command::Seek(this.playback.position_ms.saturating_sub(10000)),
                                    cx,
                                )
                            }),
                        ),
                    )
                    .child(time(p.position_ms))
                    .child(
                        div()
                            .id("seek-progress")
                            .relative()
                            .flex_1()
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .child(
                                canvas(
                                    move |bounds, _, _| painted_bounds.set(bounds),
                                    |_, _, _, _| {},
                                )
                                .absolute()
                                .size_full(),
                            )
                            .child(
                                div()
                                    .h(px(4.))
                                    .w_full()
                                    .rounded_full()
                                    .bg(theme.card)
                                    .child(
                                        div()
                                            .h_full()
                                            .w(relative(progress.clamp(0., 1.)))
                                            .bg(theme.accent),
                                    ),
                            )
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                                    let bounds = seek_bounds.get();
                                    let fraction = (f32::from(event.position.x - bounds.left())
                                        / f32::from(bounds.size.width).max(1.))
                                    .clamp(0., 1.);
                                    this.audio_command(
                                        Command::Seek(
                                            (this.playback.duration_ms as f64 * fraction as f64)
                                                as u64,
                                        ),
                                        cx,
                                    );
                                }),
                            ),
                    )
                    .child(time(p.duration_ms))
                    .child(
                        button(
                            "seek-forward",
                            "skip-forward",
                            self.t(Text::SeekForward),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.audio_command(
                                Command::Seek(this.playback.position_ms.saturating_add(10000)),
                                cx,
                            )
                        })),
                    )
                    .child(
                        button("shuffle", "shuffle", self.t(Text::Shuffle), theme)
                            .when(p.shuffle, |v| v.bg(theme.card))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.audio_command(Command::Shuffle(!this.playback.shuffle), cx)
                            })),
                    )
                    .child(
                        button(
                            "repeat",
                            "repeat",
                            format!("{} : {}", self.t(Text::Repeat), self.t(repeat)),
                            theme,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.audio_command(Command::Repeat(next_repeat), cx)
                        })),
                    ),
            )
    }
}
