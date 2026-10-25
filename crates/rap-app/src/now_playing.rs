//! In-app listening page; independent of the window's F11 fullscreen state.
use crate::{
    app::Rap,
    i18n::Text,
    widgets::{TEXT_SMALL, button, cover, quality},
};
use gpui::{prelude::*, *};

impl Rap {
    pub fn now_playing_view(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.theme();
        let track = self.playback.track.as_ref();
        let album_id = self.current_album().map(|album| album.id);
        let rem = f32::from(window.rem_size());
        // Leave room for the title bar, the persistent player, the return action and metadata.
        let side = ((f32::from(window.viewport_size().height) / rem - 26.)
            .min(crate::navigation::content_width(window) - 4.))
        .clamp(4., 32.);
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .px(crate::widgets::PAGE_INSET)
                    .pt(crate::widgets::PAGE_TOP_PADDING)
                    .pb(crate::widgets::PAGE_SECTION_GAP)
                    .flex()
                    .items_center()
                    .child(
                        button(
                            "close-now-playing",
                            "arrow-left",
                            self.t(Text::BackToLibrary),
                            theme,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.now_playing = false;
                            this.focus.focus(window);
                            cx.notify();
                        })),
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
                    .gap(rems(0.75))
                    .px(crate::widgets::PAGE_INSET)
                    .pb(crate::widgets::PAGE_SECTION_GAP)
                    .child(cover(
                        track.and_then(|t| t.cover.as_ref()),
                        side * 16.,
                        theme,
                    ))
                    .child(
                        div()
                            .w_full()
                            .max_w(rems(40.))
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(rems(0.25))
                            .child(
                                div()
                                    .w_full()
                                    .truncate()
                                    .text_center()
                                    .text_size(rems(1.5))
                                    .font_weight(FontWeight::BOLD)
                                    .child(
                                        track
                                            .map(|t| t.metadata.title.clone())
                                            .unwrap_or_else(|| self.t(Text::NothingPlaying).into()),
                                    ),
                            )
                            .when_some(track, |details, track| {
                                details
                                    .child(
                                        div()
                                            .w_full()
                                            .truncate()
                                            .text_center()
                                            .text_color(theme.accent)
                                            .id("now-artist")
                                            .tab_index(0)
                                            .border_2()
                                            .border_color(gpui::transparent_black())
                                            .capture_any_mouse_down(
                                                crate::widgets::keyboard_focus_only,
                                            )
                                            .hover(move |s| s.bg(theme.hover))
                                            .focus(move |s| {
                                                s.bg(theme.selected)
                                                    .border_2()
                                                    .border_color(theme.accent)
                                            })
                                            .on_click({
                                                let artist = track.metadata.artist.clone();
                                                cx.listener(move |this, _, window, cx| {
                                                    this.focus.focus(window);
                                                    this.now_playing = false;
                                                    this.show_artist(artist.clone(), cx);
                                                })
                                            })
                                            .child(track.metadata.artist.clone()),
                                    )
                                    .child(
                                        div()
                                            .w_full()
                                            .truncate()
                                            .text_center()
                                            .text_color(theme.muted)
                                            .id("now-album")
                                            .tab_index(0)
                                            .border_2()
                                            .border_color(gpui::transparent_black())
                                            .capture_any_mouse_down(
                                                crate::widgets::keyboard_focus_only,
                                            )
                                            .tab_stop(album_id.is_some())
                                            .when(album_id.is_some(), |v| {
                                                v.text_color(theme.accent)
                                                    .hover(move |s| s.bg(theme.hover))
                                            })
                                            .focus(move |s| {
                                                s.bg(theme.selected)
                                                    .border_2()
                                                    .border_color(theme.accent)
                                            })
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.focus.focus(window);
                                                if let Some(id) = album_id {
                                                    this.open_album(id, cx);
                                                }
                                            }))
                                            .child(track.metadata.album.clone()),
                                    )
                                    .child(
                                        div()
                                            .text_size(TEXT_SMALL)
                                            .text_color(theme.muted)
                                            .child(quality(&track.metadata)),
                                    )
                            }),
                    ),
            )
            .into_any_element()
    }

    pub fn now_playing_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        crate::widgets::icon_button(
            "open-now-playing",
            "maximize-2",
            self.t(Text::NowPlaying),
            self.now_playing,
            self.theme(),
        )
        .on_click(cx.listener(|this, _, window, cx| {
            this.now_playing = !this.now_playing;
            this.focus.focus(window);
            cx.notify();
        }))
    }
}
