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
    pub fn player(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let track = self.playback.track.as_ref();
        let compact = f32::from(window.viewport_size().width) / f32::from(window.rem_size()) < 44.;
        div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(rems(0.25))
            .py(rems(0.5))
            .bg(theme.panel)
            .border_t_1()
            .border_color(theme.border)
            .child(self.progress_row(cx).w_full().px(rems(1.)))
            .child(
                div()
                    .h(rems(5.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(rems(0.75))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .pl(rems(0.75))
                            .h_full()
                            .flex()
                            .items_center()
                            .gap(rems(0.75))
                            .when(!compact, |info| {
                                info.child(cover(track.and_then(|t| t.cover.as_ref()), 64., theme))
                            })
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
                                            .child(
                                                track
                                                    .map(|t| t.metadata.album.clone())
                                                    .unwrap_or_default(),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(TEXT_SMALL)
                                            .text_color(theme.muted)
                                            .child(
                                                track
                                                    .map(|t| t.metadata.artist.clone())
                                                    .unwrap_or_default(),
                                            ),
                                    ),
                            ),
                    )
                    .child(self.transport(compact, cx).flex_none())
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .pr(rems(0.75))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(rems(0.5))
                            .when(compact, |side| {
                                side.child(
                                    div()
                                        .w_full()
                                        .pl(rems(2.))
                                        .flex()
                                        .justify_end()
                                        .gap(rems(0.5))
                                        .child(self.compact_volume(cx))
                                        .child(self.now_playing_button(cx)),
                                )
                            })
                            .when(!compact, |side| {
                                side.child(
                                    div()
                                        .w_full()
                                        .pl(rems(2.5))
                                        .flex()
                                        .justify_end()
                                        .gap(rems(0.5))
                                        .child(self.volume_control(120., cx).max_w_full())
                                        .child(self.now_playing_button(cx)),
                                )
                            }),
                    ),
            )
    }
    fn compact_volume(&self, cx: &mut Context<Self>) -> Div {
        let theme = self.theme();
        div()
            .relative()
            .flex_none()
            .child(
                crate::widgets::icon_button(
                    "toggle-volume",
                    "volume-2",
                    self.t(Text::Volume),
                    self.volume_popup,
                    theme,
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| {
                        this.volume_popup = !this.volume_popup;
                        this.focus.focus(window);
                        cx.stop_propagation();
                        cx.notify();
                    }),
                ),
            )
            .when(self.volume_popup, |anchor| {
                anchor.child(
                    crate::widgets::popover(theme)
                        .id("volume-popup")
                        .absolute()
                        .right_0()
                        .bottom(rems(2.75))
                        .w(rems(14.))
                        .p(rems(0.75))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(self.volume_control(160., cx)),
                )
            })
    }
    pub fn transport(&self, compact: bool, cx: &mut Context<Self>) -> Div {
        let theme = self.theme();
        let p = &self.playback;
        let playing = p.state == PlaybackState::Playing;
        let (repeat_icon, repeat_label, next_repeat) = match p.repeat {
            Repeat::Off => ("repeat", Text::RepeatOff, Repeat::All),
            Repeat::All => ("repeat", Text::RepeatAll, Repeat::Track),
            Repeat::Track => ("repeat-1", Text::RepeatTrack, Repeat::Off),
        };
        // Equal controls on either side keep Play/Pause at the cartouche midpoint.
        div()
            .relative()
            .w(rems(if compact { 10.5 } else { 13. }))
            .h(rems(if compact { 2.5 } else { 3. }))
            .flex()
            .items_center()
            .gap(rems(if compact { 0.25 } else { 0.5 }))
            .when(self.audio.is_none(), |v| v.opacity(0.4))
            .child(
                crate::widgets::icon_button(
                    "shuffle",
                    "shuffle",
                    self.t(Text::Shuffle),
                    p.shuffle,
                    theme,
                )
                .when(compact, |button| button.size(rems(1.75)))
                .on_click(cx.listener(|this, _, _, cx| {
                    this.audio_command(Command::Shuffle(!this.playback.shuffle), cx)
                })),
            )
            .child(
                crate::widgets::icon_button(
                    "previous",
                    "skip-back",
                    self.t(Text::Previous),
                    false,
                    theme,
                )
                .when(compact, |button| button.size(rems(1.75)))
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
                .size(rems(if compact { 2.5 } else { 3. }))
                .rounded_full()
                .border_1()
                .border_color(theme.accent)
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
                .when(compact, |button| button.size(rems(1.75)))
                .on_click(cx.listener(|this, _, _, cx| this.audio_command(Command::Next, cx))),
            )
            .child(
                crate::widgets::icon_button(
                    "repeat",
                    repeat_icon,
                    self.labelled(self.t(Text::Repeat), self.t(repeat_label)),
                    p.repeat != Repeat::Off,
                    theme,
                )
                .when(compact, |button| button.size(rems(1.75)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.audio_command(Command::Repeat(next_repeat), cx)
                })),
            )
            .child(
                crate::widgets::icon_button(
                    "open-queue",
                    "list-end",
                    self.t(Text::Queue),
                    false,
                    theme,
                )
                .absolute()
                .left(relative(1.))
                .ml(rems(if compact { 0.25 } else { 0.5 }))
                .top(rems(if compact { 0.375 } else { 0.5 }))
                .when(compact, |button| button.size(rems(1.75)))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.focus.focus(window);
                    this.navigate(crate::app::Section::Queue, cx);
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
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h(rems(1.125))
                    .child(
                        self.seek_slider
                            .render(fraction, enabled, false, theme, on_change)
                            .w_full()
                            .h_full(),
                    )
                    .when(self.seek_slider.is_dragging(), |slider| {
                        slider.child(slider_bubble(fraction, time(shown), rems(1.25), theme))
                    }),
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
                div()
                    .relative()
                    .w(rems(width / 16.))
                    .flex_shrink()
                    .min_w(rems(2.))
                    .h(CONTROL_HEIGHT)
                    .child(
                        self.volume_slider
                            .render(volume, self.audio.is_some(), true, theme, on_change)
                            .w_full()
                            .h_full(),
                    )
                    .when(self.volume_slider.is_dragging(), |slider| {
                        slider.child(slider_bubble(
                            volume,
                            format!("{:.0} %", volume * 100.),
                            rems(1.75),
                            theme,
                        ))
                    }),
            )
    }
}

/// Floating value shared by position and volume sliders; outside the layout flow.
fn slider_bubble(
    fraction: f32,
    label: String,
    bottom: Rems,
    theme: crate::theme::Theme,
) -> Deferred {
    let mut background = theme.control;
    background.a = 1.;
    deferred(
        div()
            .absolute()
            .left(relative(fraction.clamp(0., 1.)))
            .ml(rems(-1.625))
            .bottom(bottom)
            .w(rems(3.25))
            .py(rems(0.125))
            .rounded(rems(0.3125))
            .border_1()
            .border_color(theme.border)
            .bg(background)
            .occlude()
            .shadow_sm()
            .text_center()
            .text_size(TEXT_CAPTION)
            .text_color(theme.text)
            .font(crate::widgets::numeric_font())
            .child(label),
    )
    .with_priority(2)
}
