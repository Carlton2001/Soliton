//! Permanent playback bar and transport controls.
use crate::{
    app::Rap,
    i18n::Text,
    slider::OnChange,
    widgets::{CONTROL_HEIGHT, TEXT_CAPTION, TEXT_SMALL, cover, time},
};
use gpui::{prelude::*, *};
use rap_audio::{Command, PlaybackState, Repeat};
use std::rc::Rc;

impl Rap {
    pub fn player(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let track = self.playback.track.as_ref();
        let subtitle = track
            .map(|t| format!("{} · {}", t.metadata.artist, t.metadata.album))
            .unwrap_or_default();
        div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(rems(0.5))
            .px(rems(1.))
            .py(rems(0.625))
            .bg(theme.panel)
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(rems(0.75))
                    .flex_wrap()
                    .child(
                        div()
                            .flex_1()
                            .min_w(rems(10.))
                            .flex()
                            .items_center()
                            .gap(rems(0.75))
                            .child(cover(track.and_then(|t| t.cover.as_ref()), 40., theme))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div().truncate().font_weight(FontWeight::SEMIBOLD).child(
                                            track.map(|t| t.metadata.title.clone()).unwrap_or_else(
                                                || {
                                                    self.t(if self.audio.is_some() {
                                                        Text::NothingPlaying
                                                    } else {
                                                        Text::AudioUnavailable
                                                    })
                                                    .into()
                                                },
                                            ),
                                        ),
                                    )
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
                        crate::widgets::button(
                            "open-queue",
                            "list-end",
                            self.t(Text::Queue),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.focus.focus(window);
                            this.navigate(crate::app::Section::Queue, cx);
                        })),
                    )
                    .child(self.volume_control(80., cx)),
            )
            .child(self.transport(cx))
            .child(self.progress_row(cx).w_full())
    }

    pub fn transport(&self, cx: &mut Context<Self>) -> Div {
        let theme = self.theme();
        let p = &self.playback;
        let (repeat_icon, repeat_label, next_repeat) = match p.repeat {
            Repeat::Off => ("repeat", Text::RepeatOff, Repeat::All),
            Repeat::All => ("repeat", Text::RepeatAll, Repeat::Track),
            Repeat::Track => ("repeat-1", Text::RepeatTrack, Repeat::Off),
        };
        let playing = p.state == PlaybackState::Playing;
        div()
            .flex()
            .items_center()
            .justify_center()
            .flex_wrap()
            .gap(rems(0.5))
            .when(self.audio.is_none(), |v| v.opacity(0.4))
            .child(
                crate::widgets::icon_button(
                    "previous",
                    "skip-back",
                    self.t(Text::Previous),
                    false,
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.audio_command(Command::Previous, cx))),
            )
            .child(
                crate::widgets::icon_button(
                    "play-pause",
                    if playing { "pause" } else { "play" },
                    self.t(if playing { Text::Pause } else { Text::Play }),
                    true,
                    theme,
                )
                .size(rems(2.5))
                .on_click(cx.listener(|this, _, _, cx| this.play_pause(cx))),
            )
            .child(
                crate::widgets::icon_button(
                    "next",
                    "skip-forward",
                    self.t(Text::Next),
                    false,
                    theme,
                )
                .on_click(cx.listener(|this, _, _, cx| this.audio_command(Command::Next, cx))),
            )
            .child(
                crate::widgets::icon_button(
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
                crate::widgets::icon_button(
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
            .child(
                div()
                    .font(crate::widgets::numeric_font())
                    .w(rems(2.5))
                    .flex()
                    .justify_end()
                    .child(time(shown)),
            )
            .child(
                self.seek_slider
                    .render(fraction, enabled, theme, on_change)
                    .flex_1()
                    .h(rems(1.125)),
            )
            .child(
                div()
                    .font(crate::widgets::numeric_font())
                    .w(rems(2.5))
                    .child(time(p.duration_ms)),
            )
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
        // The slider narrows before the buttons around it.
        div()
            .flex()
            .flex_shrink()
            .min_w_0()
            .items_center()
            .gap(rems(0.25))
            .child(
                crate::widgets::icon_button("mute", name, self.t(label), false, theme)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_mute(cx))),
            )
            .child(
                self.volume_slider
                    .render(volume, self.audio.is_some(), theme, on_change)
                    .w(rems(width / 16.))
                    .flex_shrink()
                    .min_w(rems(2.))
                    .h(CONTROL_HEIGHT),
            )
    }
}
