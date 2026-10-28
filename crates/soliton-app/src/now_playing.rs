//! Listening card: in-app page, or alone on the screen in full screen.
use crate::{
    app::Soliton,
    i18n::Text,
    theme::Theme,
    widgets::{RADIUS, RADIUS_LARGE, TEXT_CAPTION, TEXT_TITLE, cover, icon_button, quality},
};
use gpui::{prelude::*, *};

/// Cover band padding, on each side.
const BAND_PADDING: f32 = 1.5;

/// Card sizes, in rems; the card grows past `min_width` with its widest value.
struct Layout {
    width: f32,
    reserved: f32,
    max_side: f32,
    min_width: f32,
}

impl Soliton {
    pub fn now_playing_view(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let layout = Layout {
            width: crate::navigation::content_width(window),
            // The title bar, the persistent player and the card's text sections.
            reserved: 29.,
            max_side: 32.,
            min_width: 20.,
        };
        self.listening_card(layout, None, window, cx)
    }

    /// F11 page: the card alone, with its own playback controls.
    pub fn fullscreen_view(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let controls = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(rems(0.5))
            .px(rems(1.))
            .py(rems(0.75))
            .border_t_1()
            .border_color(self.theme().border)
            .child(self.progress_row(cx).w_full())
            .child(
                div()
                    .relative()
                    .w_full()
                    .flex()
                    .justify_center()
                    .child(self.transport(false, cx))
                    .child(
                        icon_button(
                            "exit-fullscreen",
                            "minimize-2",
                            self.t(Text::ExitFullscreen),
                            false,
                            self.theme(),
                        )
                        .absolute()
                        .right_0()
                        .top(rems(0.5))
                        .on_click(cx.listener(|this, _, window, _| this.leave_listening(window))),
                    ),
            );
        let layout = Layout {
            width: f32::from(window.viewport_size().width) / f32::from(window.rem_size()),
            // The card's text sections and its playback controls.
            reserved: 21.,
            max_side: 48.,
            // Keeps the exit button clear of the transport and its queue button.
            min_width: 26.,
        };
        self.listening_card(layout, Some(controls), window, cx)
    }

    /// Returns to the library from the page or from full screen.
    pub fn leave_listening(&mut self, window: &mut Window) {
        if window.is_fullscreen() {
            window.toggle_fullscreen();
        }
        self.focus.focus(window);
    }

    fn listening_card(
        &self,
        layout: Layout,
        controls: Option<Div>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme();
        let track = self.playback.track.as_ref();
        let album_id = self.current_album().map(|album| album.id);
        let height = f32::from(window.viewport_size().height) / f32::from(window.rem_size());
        let width = layout.width - 1.5;
        let side = (height - layout.reserved)
            .min(width - 2. * BAND_PADDING)
            .clamp(4., layout.max_side);
        let band = div()
            .w_full()
            .flex()
            .justify_center()
            .p(rems(BAND_PADDING))
            // GPUI clips children to a rectangle: the band rounds its own corners.
            .rounded_t(RADIUS_LARGE)
            .bg(theme.card)
            .child(cover(
                track.and_then(|t| t.cover.as_ref()),
                side * 16.,
                theme,
            ));
        let heading = div()
            .w_full()
            .flex()
            .flex_col()
            .items_center()
            .gap(rems(0.25))
            .px(rems(1.))
            .py(rems(1.))
            .border_t_1()
            .border_color(theme.border)
            .child(
                div()
                    .w_full()
                    .truncate()
                    .text_center()
                    .text_size(TEXT_TITLE)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(
                        track
                            .map(|t| t.metadata.title.clone())
                            .unwrap_or_else(|| self.t(Text::NothingPlaying).into()),
                    ),
            )
            .when_some(track, |heading, track| {
                let artist = track.metadata.artist.clone();
                heading.child(
                    link("now-artist", true, theme)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.leave_listening(window);
                            this.show_artist(artist.clone(), cx);
                        }))
                        .child(track.metadata.artist.clone()),
                )
            });
        let details = track.map(|track| {
            let year = track
                .metadata
                .year
                .map_or_else(|| "—".to_owned(), |year| year.to_string());
            div()
                .w_full()
                .flex()
                .border_t_1()
                .border_color(theme.border)
                .child(cell(
                    self.t(Text::Album),
                    link("now-album", album_id.is_some(), theme)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let Some(id) = album_id {
                                this.leave_listening(window);
                                this.open_album(id, cx);
                            }
                        }))
                        .child(track.metadata.album.clone()),
                    true,
                    theme,
                ))
                .child(cell(self.t(Text::Year), year, true, theme))
                .child(cell(
                    self.t(Text::Quality),
                    quality(&track.metadata),
                    false,
                    theme,
                ))
        });
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .items_center()
            .justify_center()
            .p(crate::widgets::PAGE_INSET)
            .child(
                div()
                    // Grows with its widest value, then ellipsizes once the page is full.
                    .min_w(rems(
                        (side + 2. * BAND_PADDING).max(layout.min_width).min(width),
                    ))
                    .max_w(rems(width))
                    .flex_none()
                    .overflow_hidden()
                    .rounded(RADIUS_LARGE)
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.panel)
                    // Lifted off the page: a close contact shadow under a diffuse one.
                    .shadow(vec![
                        BoxShadow {
                            color: theme.scrim.into(),
                            offset: point(px(0.), px(12.)),
                            blur_radius: px(40.),
                            spread_radius: px(0.),
                        },
                        BoxShadow {
                            color: hsla(0., 0., 0., 0.2),
                            offset: point(px(0.), px(2.)),
                            blur_radius: px(6.),
                            spread_radius: px(0.),
                        },
                    ])
                    .child(band)
                    .child(heading)
                    .children(details)
                    .children(controls),
            )
            .into_any_element()
    }

    pub fn now_playing_button(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let (icon, label) = if self.now_playing {
            ("minimize-2", Text::CollapsePlayer)
        } else {
            ("maximize-2", Text::ExpandPlayer)
        };
        crate::widgets::icon_button(
            "open-now-playing",
            icon,
            self.t(label),
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

/// Centred metadata link; a disabled one reads as plain muted text.
fn link(id: &'static str, enabled: bool, theme: Theme) -> Stateful<Div> {
    div()
        .id(id)
        .max_w_full()
        .px(rems(0.25))
        .truncate()
        .text_center()
        .text_color(theme.muted)
        .rounded(RADIUS)
        .tab_index(0)
        .tab_stop(enabled)
        .border_2()
        .border_color(gpui::transparent_black())
        .capture_any_mouse_down(crate::widgets::keyboard_focus_only)
        .when(enabled, |link| {
            link.text_color(theme.accent)
                .hover(move |s| s.bg(theme.hover))
        })
        .focus(move |s| s.bg(theme.selected).border_color(theme.accent))
}

fn cell(
    label: impl Into<SharedString>,
    value: impl IntoElement,
    separator: bool,
    theme: Theme,
) -> Div {
    div()
        .flex_auto()
        .min_w_0()
        .flex()
        .flex_col()
        .items_center()
        .text_center()
        .gap(rems(0.375))
        .px(rems(0.75))
        .py(rems(0.875))
        .when(separator, |cell| {
            cell.border_r_1().border_color(theme.border)
        })
        .child(
            div()
                .text_size(TEXT_CAPTION)
                .text_color(theme.muted)
                .child(label.into()),
        )
        .child(div().w_full().truncate().child(value))
}
