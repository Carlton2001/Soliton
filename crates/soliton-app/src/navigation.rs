//! Section rail on the left of the window: icons only, their names and shortcuts in tooltips.
use crate::{
    app::{Section, Soliton},
    i18n::Text,
    widgets::{nav_entry, nav_icon},
};
use gpui::{prelude::*, *};

/// Width of the rail, in rems; pages share the rest of the window.
pub const RAIL_WIDTH: f32 = 3.5;

/// Width left to pages beside the rail, in rems.
pub fn content_width(window: &Window) -> f32 {
    let width = window.viewport_size().width - crate::window_chrome::horizontal_inset(window);
    f32::from(width) / f32::from(window.rem_size()) - RAIL_WIDTH
}

impl Soliton {
    pub fn navigation(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                Some(crate::shortcuts::section_keys(index)),
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
        let corners = crate::window_chrome::inner_corners(window);
        let top_left = Corners {
            top_left: true,
            ..Default::default()
        };
        crate::window_chrome::round(div(), corners, top_left)
            .w(rems(RAIL_WIDTH))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(rems(0.25))
            .bg(theme.background)
            .child(
                nav_entry(
                    "navigation-brand",
                    img(crate::assets::Assets::logo_path(self.settings.app_icon))
                        .size(px(24.))
                        .flex_none(),
                    self.t(Text::Settings),
                    Some(crate::shortcuts::section_keys(4)),
                    self.section == Section::Settings,
                    theme,
                )
                .mb(rems(0.25))
                .on_click(cx.listener(|this, _, window, cx| {
                    this.focus.focus(window);
                    this.navigate(Section::Settings, cx);
                })),
            )
            .child(entry(0, Section::Artists, "users", Text::Artists, cx))
            .child(entry(1, Section::Albums, "disc-3", Text::Albums, cx))
            .child(entry(2, Section::Playlists, "list", Text::Playlists, cx))
            .child(entry(3, Section::Queue, "list-end", Text::Queue, cx))
            .when(self.updates.is_available(), |rail| {
                rail.child(
                    nav_icon(
                        "nav-info",
                        "info",
                        self.t(self.update_tooltip()),
                        None,
                        false,
                        crate::theme::Theme {
                            muted: theme.accent,
                            ..theme
                        },
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.install_update(cx))),
                )
            })
    }
}
