//! Albums stay visible while their selected record is inspected on wide windows.
use crate::app::{Rap, Section};
use gpui::{prelude::*, *};

pub(super) const ALBUM_PANEL_WIDTH: f32 = 25.;
const SPLIT_MIN_WIDTH: f32 = 72.;

pub(super) fn split_collection(width_in_rems: f32, section: Section, selected: bool) -> bool {
    section == Section::Albums && selected && width_in_rems >= SPLIT_MIN_WIDTH
}

impl Rap {
    pub(super) fn collection_view(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let split = split_collection(
            f32::from(window.viewport_size().width) / f32::from(window.rem_size()),
            self.section,
            self.selected.is_some(),
        );
        if self.selected.is_some() && !split {
            return self.album_view(window, cx);
        }
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .flex()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(self.header(cx))
                    .child(self.albums_view(cx)),
            )
            .when(split, |view| {
                view.child(
                    div()
                        .w(rems(ALBUM_PANEL_WIDTH))
                        .h_full()
                        .flex_none()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .bg(self.theme().panel)
                        .child(self.album_panel(window, cx)),
                )
            })
            .into_any_element()
    }
}

#[cfg(test)]
#[path = "tests/collection.rs"]
mod tests;
