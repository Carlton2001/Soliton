//! Permanent playback bar, and controls shared with the now playing view.
use crate::{
    app::Rap,
    i18n::Text,
    slider::OnChange,
    widgets::{cover, time, tool},
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
            .gap_4()
            .px_4()
            .py_3()
            .bg(theme.panel)
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .id("current-track")
                    .w(px(280.))
                    .flex_shrink_0()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_3()
                    .rounded_md()
                    .when(track.is_some(), |view| {
                        view.hover(move |s| s.bg(theme.card)).on_click(cx.listener(
                            |this, _, _, cx| {
                                this.now_playing = true;
                                cx.notify();
                            },
                        ))
                    })
                    .child(cover(track.and_then(|t| t.cover.as_ref()), 52., theme))
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
                                    .text_xs()
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
                    .gap_1()
                    .child(self.transport(false, cx))
                    .child(self.progress_row(cx).w_full().max_w(px(720.))),
            )
            .child(
                div()
                    .w(px(240.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_1()
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
        let main = tool(
            "play-pause",
            if playing { "pause" } else { "play" },
            self.t(if playing { Text::Pause } else { Text::Play }),
            false,
            theme,
        )
        .bg(theme.card)
        .rounded_full()
        .p(px(if large { 16. } else { 10. }));
        div()
            .flex()
            .items_center()
            .gap(px(if large { 20. } else { 8. }))
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
            .gap_3()
            .text_xs()
            .text_color(theme.muted)
            .child(div().w(px(40.)).flex().justify_end().child(time(shown)))
            .child(
                self.seek_slider
                    .render(fraction, enabled, theme, on_change)
                    .flex_1()
                    .h(px(18.)),
            )
            .child(div().w(px(40.)).child(time(p.duration_ms)))
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
            .gap_1()
            .child(
                tool("mute", name, self.t(label), false, theme)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_mute(cx))),
            )
            .child(
                self.volume_slider
                    .render(volume, self.audio.is_some(), theme, on_change)
                    .w(px(width))
                    .h(px(18.)),
            )
    }
}
