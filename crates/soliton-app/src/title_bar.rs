use crate::app::Soliton;
use gpui::{prelude::*, *};

pub(crate) const TITLE_BAR_HEIGHT: f32 = 32.;
const CAPTION_BUTTON_WIDTH: f32 = 46.;
/// Hover color of the close caption button, as in native Windows title bars.
const CLOSE_HOVER: u32 = 0xc42b1c;

impl Soliton {
    /// Title bar drawn in the application theme; the native one is transparent.
    pub fn title_bar(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let hover = theme.card;
        let drag = crate::window_chrome::drag_region("title-bar-drag", window, cx)
            .h_full()
            .flex()
            .items_center()
            .flex_1();
        // Caption symbols come from Oxyde: a 12-unit grid with a one-unit stroke.
        let caption = |id: &'static str, area: WindowControlArea, icon: &'static str| {
            div()
                .id(id)
                .w(px(CAPTION_BUTTON_WIDTH))
                .h_full()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .window_control_area(area)
                .when(cfg!(target_os = "linux"), |button| {
                    // Acts on release inside the button, like native caption buttons.
                    button.on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                        if !event.standard_click() {
                            return;
                        }
                        match area {
                            WindowControlArea::Min => window.minimize_window(),
                            WindowControlArea::Max => window.zoom_window(),
                            WindowControlArea::Close => this.request_close(window, cx),
                            WindowControlArea::Drag => {}
                        }
                    }))
                })
                .child(svg().path(icon).size(px(12.)).text_color(theme.text))
        };
        let top_right = Corners {
            top_right: true,
            ..Default::default()
        };
        let corners = crate::window_chrome::inner_corners(window);
        let maximize_icon = if window.is_maximized() {
            "window-controls/restore.svg"
        } else {
            "window-controls/maximize.svg"
        };
        crate::window_chrome::round(div(), corners, top_right)
            .id("title-bar")
            .w_full()
            .h(px(TITLE_BAR_HEIGHT))
            .flex()
            .flex_none()
            .items_center()
            .bg(theme.background)
            .child(drag)
            // Windows handles these clicks natively through their control areas.
            .child(
                caption(
                    "title-minimize",
                    WindowControlArea::Min,
                    "window-controls/minimize.svg",
                )
                .hover(move |style| style.bg(hover)),
            )
            .child(
                caption("title-maximize", WindowControlArea::Max, maximize_icon)
                    .hover(move |style| style.bg(hover)),
            )
            .child(
                crate::window_chrome::round(
                    caption(
                        "title-close",
                        WindowControlArea::Close,
                        "window-controls/close.svg",
                    ),
                    corners,
                    top_right,
                )
                .hover(|style| style.bg(rgb(CLOSE_HOVER))),
            )
    }
}
