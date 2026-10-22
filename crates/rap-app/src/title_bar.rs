use crate::{
    app::{Rap, Section},
    i18n::Text,
    widgets::{nav_item, tool},
};
use gpui::{prelude::*, *};

const TITLE_BAR_HEIGHT: f32 = 36.;
const CAPTION_BUTTON_WIDTH: f32 = 46.;
/// Hover color of the close caption button, as in native Windows title bars.
const CLOSE_HOVER: u32 = 0xc42b1c;

impl Rap {
    /// Title bar drawn in the application theme; the native one is transparent.
    pub fn title_bar(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        let drag = |id: &'static str| {
            div()
                .id(id)
                .h_full()
                .flex()
                .items_center()
                .window_control_area(WindowControlArea::Drag)
                .on_mouse_down(MouseButton::Middle, |_, window, _| window.minimize_window())
        };
        let entries = [
            (Section::Artists, "users", Text::Artists),
            (Section::Albums, "disc-3", Text::Albums),
        ];
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
                drag("title-bar-brand")
                    .flex_none()
                    .gap_2()
                    .pl_3()
                    .pr_4()
                    .child(img("images/logo.svg").size(px(18.)).flex_none())
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::BOLD)
                            .child("RAP"),
                    ),
            )
            // Outside the drag areas so that Windows lets the clicks through.
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(2.))
                    .children(entries.into_iter().enumerate().map(
                        |(index, (section, icon, label))| {
                            nav_item(
                                ("nav", index),
                                icon,
                                self.t(label),
                                !self.now_playing && self.section == section,
                                theme,
                            )
                            .h(px(TITLE_BAR_HEIGHT - 8.))
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.focus.focus(window);
                                    this.now_playing = false;
                                    this.navigate(section, cx);
                                },
                            ))
                        },
                    )),
            )
            .child(drag("title-bar-drag").flex_1())
            .child(
                tool(
                    "nav-settings",
                    "settings",
                    self.t(Text::Settings),
                    !self.now_playing && self.section == Section::Settings,
                    theme,
                )
                .size(px(TITLE_BAR_HEIGHT - 8.))
                .mr_1()
                .on_click(cx.listener(|this, _, window, cx| {
                    this.focus.focus(window);
                    this.now_playing = false;
                    this.navigate(Section::Settings, cx);
                })),
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
