//! Immersive playback view: large cover and essential controls. It can also put
//! the window in full screen (F11, Escape to leave).
use crate::{
    app::Rap,
    i18n::Text,
    widgets::{button, cover, quality, tool},
};
use gpui::{prelude::*, *};

impl Rap {
    pub fn now_playing_view(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let viewport = window.viewport_size();
        // The cover takes what the text and controls leave, within reason.
        let side = (f32::from(viewport.height) - 360.)
            .min(f32::from(viewport.width) * 0.5)
            .clamp(160., 640.);
        let fullscreen = window.is_fullscreen();
        let track = self.playback.track.as_ref();
        // The album sheet opens on the edition being played.
        let album = self.current_album().zip(track).map(|(album, track)| {
            let edition = album
                .editions
                .iter()
                .position(|e| e.tracks.iter().any(|t| t.id == track.id))
                .unwrap_or(0);
            (album.id, edition)
        });
        let mut details = track.map(|t| quality(&t.metadata)).unwrap_or_default();
        if let Some(rate) = self.playback.output_rate.filter(|_| track.is_some()) {
            details.push_str(&format!(" → {} {rate} Hz", self.t(Text::Output)));
        }
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .child(
                        button("collapse", "chevron-down", self.t(Text::Collapse), theme).on_click(
                            cx.listener(|this, _, window, cx| {
                                if window.is_fullscreen() {
                                    window.toggle_fullscreen();
                                }
                                this.now_playing = false;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        tool(
                            "toggle-fullscreen",
                            if fullscreen {
                                "minimize-2"
                            } else {
                                "maximize-2"
                            },
                            self.t(if fullscreen {
                                Text::ExitFullScreen
                            } else {
                                Text::FullScreen
                            }),
                            false,
                            theme,
                        )
                        .on_click(cx.listener(|_, _, window, _| window.toggle_fullscreen())),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap_4()
                    .px_8()
                    .child(cover(track.and_then(|t| t.cover.as_ref()), side, theme))
                    .child(
                        div()
                            .max_w(px(side.max(480.)))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .text_3xl()
                                    .font_weight(FontWeight::BOLD)
                                    .truncate()
                                    .child(
                                        track
                                            .map(|t| t.metadata.title.clone())
                                            .unwrap_or_else(|| self.t(Text::NothingPlaying).into()),
                                    ),
                            )
                            .when_some(track, |column, track| {
                                let artist = track.metadata.artist.clone();
                                column
                                    .child(
                                        div()
                                            .id("now-artist")
                                            .text_xl()
                                            .text_color(theme.accent)
                                            .truncate()
                                            .rounded_md()
                                            .px_2()
                                            .hover(move |s| s.bg(theme.card))
                                            .child(artist.clone())
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.show_artist(artist.clone(), cx)
                                            })),
                                    )
                                    .child(
                                        div()
                                            .text_color(theme.muted)
                                            .truncate()
                                            .child(track.metadata.album.clone()),
                                    )
                                    .child(div().text_xs().text_color(theme.muted).child(details))
                            }),
                    )
                    .child(self.progress_row(cx).w_full().max_w(px(640.)))
                    .child(self.transport(true, cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_4()
                            .child(self.volume_control(160., cx))
                            .when_some(album, |row, (id, edition)| {
                                row.child(
                                    button("go-to-album", "disc-3", self.t(Text::GoToAlbum), theme)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            if window.is_fullscreen() {
                                                window.toggle_fullscreen();
                                            }
                                            this.open_album(id, cx);
                                            this.selected_edition = edition;
                                        })),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }
}
