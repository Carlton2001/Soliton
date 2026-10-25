//! Section rail on the left of the window: icons only, their names in tooltips.
use crate::{
    app::{Rap, Section},
    i18n::Text,
    widgets::nav_icon,
};
use gpui::{prelude::*, *};

/// Width of the rail, in rems; pages share the rest of the window.
pub const RAIL_WIDTH: f32 = 3.5;

/// Width left to pages beside the rail, in rems.
pub fn content_width(window: &Window) -> f32 {
    f32::from(window.viewport_size().width) / f32::from(window.rem_size()) - RAIL_WIDTH
}

impl Rap {
    pub fn navigation(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let entry = |index: usize,
                     section: Section,
                     icon: &'static str,
                     label: Text,
                     cx: &mut Context<Self>| {
            nav_icon(
                ("nav", index),
                icon,
                self.t(label),
                self.section == section,
                theme,
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.focus.focus(window);
                if section == Section::Playlists {
                    this.playlists.selected = None;
                }
                this.navigate(section, cx);
            }))
        };
        div()
            .w(rems(RAIL_WIDTH))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .gap(rems(0.25))
            .py(rems(0.5))
            .bg(theme.background)
            .child(entry(0, Section::Artists, "users", Text::Artists, cx))
            .child(entry(1, Section::Albums, "disc-3", Text::Albums, cx))
            .child(entry(2, Section::Playlists, "list", Text::Playlists, cx))
            .child(entry(3, Section::Queue, "list-end", Text::Queue, cx))
            .child(div().flex_1())
            .child(entry(4, Section::Settings, "settings", Text::Settings, cx))
    }
}
