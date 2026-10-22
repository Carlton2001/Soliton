//! Permanent playback bar, and controls shared with the now playing view.
use crate::{
    app::Rap,
    i18n::Text,
    slider::OnChange,
    widgets::{
        CONTROL_HEIGHT, TEXT_CAPTION, TEXT_SMALL, cover, time, tinted_icon, tool, with_tooltip,
    },
};
use gpui::{prelude::*, *};
use rap_audio::{Command, PlaybackState, Repeat};
use std::rc::Rc;

impl Rap {
    pub fn player(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let track = self.playback.track.as_ref();
        let subtitle = track
            .map(|t| {
                [&t.metadata.artist, &t.metadata.album]
                    .into_iter()
                    .filter(|v| !v.is_empty())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .unwrap_or_default();
        let enabled = self.audio.is_some();
        div()
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(rems(1.))
            .px(rems(1.))
            .py(rems(0.625))
            .bg(theme.panel)
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .id("current-track")
                    .w(rems(17.5))
                    .flex_shrink_0()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(rems(0.75))
                    .p(rems(0.25))
                    .rounded(crate::widgets::RADIUS_LARGE)
                    .when(track.is_some(), |view| {
                        view.hover(move |s| s.bg(theme.hover))
                            .active(move |s| s.bg(theme.pressed))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.now_playing = true;
                                cx.notify();
                            }))
                    })
                    .child(cover(track.and_then(|t| t.cover.as_ref()), 48., theme))
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().truncate().font_weight(FontWeight::SEMIBOLD).child(
                                track.map(|t| t.metadata.title.clone()).unwrap_or_else(|| {
                                    self.t(if enabled {
                                        Text::NothingPlaying
                                    } else {
                                        Text::AudioUnavailable
                                    })
                                    .into()
                                }),
                            ))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(TEXT_SMALL)
                                    .text_color(theme.muted)
                                    .child(subtitle),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(rems(0.25))
                    .child(self.transport(false, cx))
                    .child(self.progress_row(cx).w_full().max_w(rems(45.))),
            )
            .child(
                div()
                    .w(rems(15.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(rems(0.25))
                    .child(self.volume_control(110., cx))
                    .child(
                        tool(
                            "open-now-playing",
                            "maximize-2",
                            self.t(Text::NowPlaying),
                            false,
                            theme,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.now_playing = true;
                            cx.notify();
                        })),
                    ),
            )
    }

    /// Shuffle, previous, play or pause, next and repeat.
    pub fn transport(&self, large: bool, cx: &mut Context<Self>) -> Div {
        let theme = self.theme();
        let p = &self.playback;
        let (repeat_icon, repeat_label) = match p.repeat {
            Repeat::Off => ("repeat", Text::RepeatOff),
            Repeat::All => ("repeat", Text::RepeatAll),
            Repeat::Track => ("repeat-1", Text::RepeatTrack),
        };
        let next_repeat = match p.repeat {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::Track,
            Repeat::Track => Repeat::Off,
        };
        let playing = p.state == PlaybackState::Playing;
        // The main action stands out with the accent fill.
        let main = with_tooltip(
            div()
                .id("play-pause")
                .size(if large { rems(3.5) } else { rems(2.5) })
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded_full()
                .bg(theme.accent)
                .hover(|s| s.opacity(0.9))
                .active(|s| s.opacity(0.8))
                .child(
                    tinted_icon(if playing { "pause" } else { "play" }, theme.on_accent)
                        .size(if large { rems(1.5) } else { rems(1.125) }),
                ),
            self.t(if playing { Text::Pause } else { Text::Play })
                .into(),
            theme,
        );
        div()
            .flex()
            .items_center()
            .gap(if large { rems(1.25) } else { rems(0.5) })
            .when(self.audio.is_none(), |row| row.opacity(0.4))
            .child(
                tool(
                    "shuffle",
                    "shuffle",
                    self.t(Text::Shuffle),
                    p.shuffle,
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.audio_command(Command::Shuffle(!this.playback.shuffle), cx)
                })),
            )
            .child(
                tool(
                    "previous",
                    "skip-back",
                    self.t(Text::Previous),
                    false,
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.audio_command(Command::Previous, cx))),
            )
            .child(main.on_click(cx.listener(|this, _, _, cx| this.play_pause(cx))))
            .child(
                tool("next", "skip-forward", self.t(Text::Next), false, theme)
                    .on_click(cx.listener(|this, _, _, cx| this.audio_command(Command::Next, cx))),
            )
            .child(
                tool(
                    "repeat",
                    repeat_icon,
                    self.labelled(self.t(Text::Repeat), self.t(repeat_label)),
                    p.repeat != Repeat::Off,
                    theme,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.audio_command(Command::Repeat(next_repeat), cx)
                })),
            )
    }

    /// Elapsed time, draggable position and duration.
    pub fn progress_row(&self, cx: &mut Context<Self>) -> Div {
        let theme = self.theme();
        let p = &self.playback;
        let fraction = self.seek_preview.unwrap_or(if p.duration_ms > 0 {
            p.position_ms as f32 / p.duration_ms as f32
        } else {
            0.
        });
        let shown = match self.seek_preview {
            Some(preview) => (p.duration_ms as f32 * preview) as u64,
            None => p.position_ms,
        };
        let entity = cx.weak_entity();
        let on_change: OnChange = Rc::new(move |fraction, released, _, cx| {
            let _ = entity.update(cx, |this, cx| this.seek_fraction(fraction, released, cx));
        });
        let enabled = p.track.is_some() && p.duration_ms > 0;
        div()
            .flex()
            .items_center()
            .gap(rems(0.75))
            .text_size(TEXT_CAPTION)
            .text_color(theme.muted)
            .child(div().w(rems(2.5)).flex().justify_end().child(time(shown)))
            .child(
                self.seek_slider
                    .render(fraction, enabled, theme, on_change)
                    .flex_1()
                    .h(rems(1.125)),
            )
            .child(div().w(rems(2.5)).child(time(p.duration_ms)))
    }

    /// Mute toggle and draggable volume.
    pub fn volume_control(&self, width: f32, cx: &mut Context<Self>) -> Div {
        let theme = self.theme();
        let volume = self.settings.volume;
        let (name, label) = if volume == 0. {
            ("volume-x", Text::Unmute)
        } else if volume < 0.5 {
            ("volume-1", Text::Mute)
        } else {
            ("volume-2", Text::Mute)
        };
        let entity = cx.weak_entity();
        let on_change: OnChange = Rc::new(move |fraction, released, _, cx| {
            let _ = entity.update(cx, |this, cx| this.set_volume(fraction, released, cx));
        });
        div()
            .flex()
            .items_center()
            .gap(rems(0.25))
            .child(
                tool("mute", name, self.t(label), false, theme)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_mute(cx))),
            )
            .child(
                self.volume_slider
                    .render(volume, self.audio.is_some(), theme, on_change)
                    .w(rems(width / 16.))
                    .h(CONTROL_HEIGHT),
            )
    }
}
