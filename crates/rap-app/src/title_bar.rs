use crate::app::Rap;
use gpui::*;

const TITLE_BAR_HEIGHT: f32 = 36.;
const CAPTION_BUTTON_WIDTH: f32 = 46.;
/// Hover color of the close caption button, as in native Windows title bars.
const CLOSE_HOVER: u32 = 0xc42b1c;

impl Rap {
    /// Title bar drawn in the application theme; the native one is transparent.
    pub fn title_bar(&self, window: &Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let hover = theme.card;
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
                .child(svg().path(icon).size(px(14.)).text_color(theme.text))
        };
        let maximize_icon = if window.is_maximized() {
            "icons/copy.svg"
        } else {
            "icons/square.svg"
        };
        div()
            .id("title-bar")
            .w_full()
            .h(px(TITLE_BAR_HEIGHT))
            .flex()
            .flex_none()
            .items_center()
            .bg(theme.panel)
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .id("title-bar-drag")
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .pl_3()
                    .window_control_area(WindowControlArea::Drag)
                    .on_mouse_down(MouseButton::Middle, |_, window, _| window.minimize_window())
                    .child(img("images/logo.svg").size(px(18.)).flex_none())
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::BOLD)
                            .child("RAP"),
                    ),
            )
            // Windows handles these clicks natively through their control areas.
            .child(
                caption("title-minimize", WindowControlArea::Min, "icons/minus.svg")
                    .hover(move |style| style.bg(hover)),
            )
            .child(
                caption("title-maximize", WindowControlArea::Max, maximize_icon)
                    .hover(move |style| style.bg(hover)),
            )
            .child(
                caption("title-close", WindowControlArea::Close, "icons/x.svg")
                    .hover(|style| style.bg(rgb(CLOSE_HOVER))),
            )
    }
}
