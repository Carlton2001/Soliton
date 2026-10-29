//! Permanent playback bar and transport controls.
use crate::{
    app::Soliton,
    i18n::Text,
    slider::OnChange,
    widgets::{CONTROL_HEIGHT, TEXT_CAPTION, TEXT_SMALL, cover, time},
};
use gpui::{prelude::*, *};
use soliton_audio::{Command, PlaybackState, Repeat};
use std::rc::Rc;

impl Soliton {
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
            .bg(theme.background)
            .border_t_1()
            .border_color(theme.border)
            .child(self.progress_row(cx).w_full().px(rems(0.75)))
            .child(
                div()
                    .h(rems(4.))
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
                                info.child(cover(track.and_then(|t| t.cover.as_ref()), 52., theme))
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .w_full()
                                            .truncate()
                                            .whitespace_normal()
                                            .line_clamp(1)
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(
                                                track
                                                    .map(|t| t.metadata.title.clone())
                                                    .unwrap_or_else(|| {
                                                        self.t(if self.audio.is_some() {
                                                            Text::NothingPlaying
                                                        } else {
                                                            Text::AudioUnavailable
                                                        })
                                                        .into()
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .truncate()
                                            .whitespace_normal()
                                            .line_clamp(1)
                                            .text_size(TEXT_SMALL)
                                            .text_color(theme.muted)
                                            .child(
                                                track
                                                    .map(|t| self.track_subtitle(&t.metadata))
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
                                        .when(
                                            f32::from(window.viewport_size().width)
                                                / f32::from(window.rem_size())
                                                >= 90.,
                                            |side| side.child(self.output_control(cx)),
                                        )
                                        .child(self.now_playing_button(cx)),
                                )
                            }),
                    ),
            )
    }
    fn output_control(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = self.theme();
        let name = self
            .playback
            .network_output
            .as_ref()
            .map(|output| output.name.clone())
            .or_else(|| {
                self.playback
                    .output
                    .as_ref()
                    .map(|output| self.output_label(output))
            })
            .unwrap_or_else(|| self.t(Text::AudioOutput).into());
        crate::widgets::with_tooltip(
            div()
                .id("player-output")
                .tab_index(0)
                .capture_any_mouse_down(crate::widgets::keyboard_focus_only)
                .h(CONTROL_HEIGHT)
                .max_w(rems(10.))
                .min_w_0()
                .px(rems(0.625))
                .flex()
                .items_center()
                .gap(rems(0.5))
                .rounded(crate::widgets::RADIUS)
                .border_1()
                .border_color(theme.control_border)
                .bg(theme.control)
                .text_color(theme.text)
                .text_size(TEXT_SMALL)
                .focus(move |style| style.border_color(theme.accent))
                .hover(move |style| style.bg(theme.control_hover))
                .active(move |style| style.bg(theme.control_pressed))
                .child(crate::widgets::tinted_icon("volume-2", theme.text))
                .child(
                    div()
                        .min_w_0()
                        .w_full()
                        .truncate()
                        .whitespace_normal()
                        .line_clamp(1)
                        .child(name.clone()),
                )
                .on_click(cx.listener(|this, _, window, cx| {
                    this.focus.focus(window);
                    this.navigate(crate::app::Section::Settings, cx);
                })),
            name.into(),
            theme,
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
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(cx.listener(|this, _, _, cx| {
                    this.volume_popup = !this.volume_popup;
                    cx.stop_propagation();
                    cx.notify();
                })),
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
        let soliton = cx.weak_entity();
        let play_pause_label = move |cx: &App| -> SharedString {
            soliton
                .upgrade()
                .map(|soliton| {
                    let soliton = soliton.read(cx);
                    let playing = soliton.playback.state == PlaybackState::Playing;
                    soliton
                        .t(if playing { Text::Pause } else { Text::Play })
                        .into()
                })
                .unwrap_or_default()
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
                .on_click(
                    cx.listener(|this, _, _, cx| this.set_shuffle(!this.playback.shuffle, cx)),
                ),
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
                crate::widgets::accent_icon_button(
                    "play-pause",
                    if playing { "pause" } else { "play" },
                    play_pause_label,
                    theme,
                )
                .size(rems(if compact { 2.5 } else { 3. }))
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
                .on_click(cx.listener(move |this, _, _, cx| this.set_repeat(next_repeat, cx))),
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
                    this.leave_listening(window);
                    this.navigate(crate::app::Section::Queue, cx);
                })),
            )
    }
    /// Elapsed time, draggable position and duration.
    pub fn progress_row(&self, cx: &mut Context<Self>) -> Div {
        let theme = self.theme();
        let p = &self.playback;
        let position = self.shown_position();
        let fraction = self.seek_preview.unwrap_or(if p.duration_ms > 0 {
            position as f32 / p.duration_ms as f32
        } else {
            0.
        });
        let shown = match self.seek_preview {
            Some(preview) => (p.duration_ms as f32 * preview) as u64,
            None => position,
        };
        let entity = cx.weak_entity();
        let on_change: OnChange = Rc::new(move |fraction, released, _, cx| {
            let _ = entity.update(cx, |this, cx| this.seek_fraction(fraction, released, cx));
        });
        let enabled = p.track.is_some() && p.duration_ms > 0;
        // Both clocks widen together for h:mm:ss, so the slider never shifts mid-track.
        let clock = rems(if p.duration_ms >= 3_600_000 {
            3.25
        } else {
            2.5
        });
        div()
            .flex()
            .items_center()
            .gap(rems(0.75))
            .text_size(TEXT_CAPTION)
            .text_color(theme.muted)
            .child(
                div()
                    .font(crate::widgets::numeric_font())
                    .w(clock)
                    .flex()
                    .justify_end()
                    .child(time(shown)),
            )
            .child(
                div()
                    .relative()
                    .id("seek-keyboard")
                    .tab_index(0)
                    .tab_stop(enabled)
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .focus(move |s| s.border_color(theme.accent))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if !enabled {
                            return;
                        }
                        if let Some(value) =
                            crate::slider::keyboard_value(&event.keystroke.key, fraction, 0.02)
                        {
                            this.seek_fraction(value, true, cx);
                            cx.stop_propagation();
                        }
                    }))
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
                    .w(clock)
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
                    .id("volume-keyboard")
                    .tab_index(0)
                    .tab_stop(self.audio.is_some())
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .focus(move |s| s.border_color(theme.accent))
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                        if let Some(value) = crate::slider::keyboard_value(
                            &event.keystroke.key,
                            this.settings.volume,
                            0.05,
                        ) {
                            this.set_volume(value, true, cx);
                            cx.stop_propagation();
                        }
                    }))
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
    // Opaque like tooltips: the translucent control fill has no surface behind it here.
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
            .border_color(theme.control_border)
            .bg(theme.panel)
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
