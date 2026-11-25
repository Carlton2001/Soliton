use crate::{app::Soliton, theme::Theme, window_chrome};
use gpui::{prelude::*, *};
use soliton_platform::{CaptionButton, CaptionStyle};

pub(crate) const TITLE_BAR_HEIGHT: f32 = 32.;
const CAPTION_BUTTON_WIDTH: f32 = 46.;
const CLOSE_HOVER: u32 = 0xc42b1c;

impl Soliton {
    pub fn title_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drag = window_chrome::drag_region("title-bar-drag", window, cx)
            .h_full()
            .flex_1();
        let style = self.window_controls.style;
        let theme = self.theme();
        let corners = window_chrome::inner_corners(window);
        let maximized = window.is_maximized();
        let active = window.is_window_active();
        let group = |buttons: &[CaptionButton], left: bool| {
            div()
                .h_full()
                .flex()
                .flex_none()
                .items_center()
                .when(
                    !buttons.is_empty() && style != CaptionStyle::Windows,
                    |row| {
                        let edge = if style == CaptionStyle::Adwaita {
                            px(2.)
                        } else {
                            px(7.)
                        };
                        if left {
                            row.pl(edge).pr(edge)
                        } else {
                            row.pr(edge)
                        }
                    },
                )
                .children(buttons.iter().enumerate().map(|(index, &button)| {
                    let outer_left = left && index == 0;
                    let outer_right = !left && index + 1 == buttons.len();
                    let (id, area) = match button {
                        CaptionButton::Minimize => ("title-minimize", WindowControlArea::Min),
                        CaptionButton::Maximize => ("title-maximize", WindowControlArea::Max),
                        CaptionButton::Close => ("title-close", WindowControlArea::Close),
                    };
                    let shell = div()
                        .id(id)
                        .h_full()
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .window_control_area(area);
                    if style == CaptionStyle::Windows {
                        let hover = if button == CaptionButton::Close {
                            rgb(CLOSE_HOVER)
                        } else {
                            theme.card
                        };
                        return window_chrome::round(
                            shell,
                            corners,
                            Corners {
                                top_left: outer_left,
                                top_right: outer_right,
                                ..Default::default()
                            },
                        )
                        .w(px(CAPTION_BUTTON_WIDTH))
                        .hover(move |s| s.bg(hover))
                        .child(
                            svg()
                                .path(caption_icon(style, button, maximized))
                                .size(px(12.))
                                .text_color(theme.text),
                        );
                    }

                    shell
                        .group(id)
                        .w(if style == CaptionStyle::Adwaita {
                            px(36.)
                        } else {
                            px(24.)
                        })
                        .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                            if event.standard_click() {
                                match button {
                                    CaptionButton::Minimize => window.minimize_window(),
                                    CaptionButton::Maximize => window.zoom_window(),
                                    CaptionButton::Close => this.request_close(window, cx),
                                }
                            }
                        }))
                        .child(linux_disc(id, style, button, maximized, active, theme))
                }))
        };
        window_chrome::round(
            div(),
            corners,
            Corners {
                top_left: true,
                top_right: true,
                ..Default::default()
            },
        )
        .id("title-bar")
        .w_full()
        .h(px(TITLE_BAR_HEIGHT))
        .flex()
        .flex_none()
        .items_center()
        .child(group(&self.window_controls.left, true))
        .child(drag)
        .child(group(&self.window_controls.right, false))
    }
}

fn linux_disc(
    group: &'static str,
    style: CaptionStyle,
    button: CaptionButton,
    maximized: bool,
    active: bool,
    theme: Theme,
) -> Stateful<Div> {
    let adwaita = style == CaptionStyle::Adwaita;
    let red_close = !adwaita && button == CaptionButton::Close;
    let text = Hsla::from(theme.text);
    let color = if active { theme.text } else { theme.muted };
    let hover = if red_close {
        rgb(CLOSE_HOVER).into()
    } else {
        text.opacity(0.15)
    };
    let pressed = if red_close {
        rgb(0xa82318).into()
    } else {
        text.opacity(0.25)
    };
    let glyph_size = match (style, button, maximized) {
        (CaptionStyle::Breeze, CaptionButton::Close, _) => 8.,
        (CaptionStyle::Breeze, CaptionButton::Maximize, true) => 12.,
        _ => 16.,
    };
    div()
        .id("disc")
        .size(px(if adwaita { 24. } else { 22. }))
        .rounded_full()
        .flex()
        .items_center()
        .justify_center()
        .when(adwaita, |disc| disc.bg(text.opacity(0.1)))
        .group_hover(group, move |s| s.bg(hover))
        .group_active(group, move |s| s.bg(pressed))
        .child(
            svg()
                .path(caption_icon(style, button, maximized))
                .size(px(glyph_size))
                .text_color(color)
                .when(red_close, |icon| {
                    icon.group_hover(group, |s| s.text_color(white()))
                }),
        )
}

fn caption_icon(style: CaptionStyle, button: CaptionButton, maximized: bool) -> &'static str {
    match (style, button, maximized) {
        (CaptionStyle::Adwaita, CaptionButton::Close, _) => "window-controls/adwaita/close.svg",
        (CaptionStyle::Adwaita, CaptionButton::Minimize, _) => {
            "window-controls/adwaita/minimize.svg"
        }
        (CaptionStyle::Adwaita, CaptionButton::Maximize, true) => {
            "window-controls/adwaita/restore.svg"
        }
        (CaptionStyle::Adwaita, CaptionButton::Maximize, false) => {
            "window-controls/adwaita/maximize.svg"
        }
        (_, CaptionButton::Close, _) => "window-controls/close.svg",
        (CaptionStyle::Breeze, CaptionButton::Minimize, _) => "icons/chevron-down.svg",
        (CaptionStyle::Breeze, CaptionButton::Maximize, true) => "icons/diamond.svg",
        (CaptionStyle::Breeze, CaptionButton::Maximize, false) => "icons/chevron-up.svg",
        (_, CaptionButton::Minimize, _) => "window-controls/minimize.svg",
        (_, CaptionButton::Maximize, true) => "window-controls/restore.svg",
        (_, CaptionButton::Maximize, false) => "window-controls/maximize.svg",
    }
}
