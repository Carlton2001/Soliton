use crate::app::Rap;
use gpui::{prelude::*, *};

const TITLE_BAR_HEIGHT: f32 = 40.;
const CAPTION_BUTTON_WIDTH: f32 = 46.;
/// Hover color of the close caption button, as in native Windows title bars.
const CLOSE_HOVER: u32 = 0xc42b1c;

impl Rap {
    /// Title bar drawn in the application theme; the native one is transparent.
    pub fn title_bar(&self, window: &Window) -> impl IntoElement {
        let theme = self.theme();
        let hover = theme.card;
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
                .child(svg().path(icon).size_3().text_color(theme.text))
        };
        let maximize_icon = if window.is_maximized() {
            "window-controls/restore.svg"
        } else {
            "window-controls/maximize.svg"
        };
        let drag = |id: &'static str| {
            div()
                .id(id)
                .h_full()
                .flex()
                .items_center()
                .window_control_area(WindowControlArea::Drag)
                .on_mouse_down(MouseButton::Middle, |_, window, _| window.minimize_window())
        };
        div()
            .id("title-bar")
            .w_full()
            .h(px(TITLE_BAR_HEIGHT))
            .flex()
            .flex_none()
            .items_center()
            .bg(theme.background)
            .child(
                drag("title-bar-brand")
                    .flex_none()
                    .gap_2()
                    .pl_3()
                    .pr_4()
                    .child(
                        img(crate::assets::Assets::logo_path(self.settings.app_icon))
                            .size(px(18.))
                            .flex_none(),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::BOLD)
                            .child("rap"),
                    ),
            )
            .child(drag("title-bar-drag").flex_1())
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
                caption(
                    "title-close",
                    WindowControlArea::Close,
                    "window-controls/close.svg",
                )
                .hover(|style| style.bg(rgb(CLOSE_HOVER))),
            )
    }
}
