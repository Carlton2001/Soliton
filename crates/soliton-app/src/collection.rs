//! Albums stay visible while their selected record is inspected on wide windows.
use crate::app::{Section, Soliton};
use gpui::{prelude::*, *};

const SPLIT_MIN_WIDTH: f32 = 72.;

/// The record and the collection share the window equally.
pub(super) fn album_panel_width(width_in_rems: f32) -> f32 {
    width_in_rems / 2.
}

pub(super) fn split_collection(width_in_rems: f32, section: Section, selected: bool) -> bool {
    section == Section::Albums && selected && width_in_rems >= SPLIT_MIN_WIDTH
}

impl Soliton {
    pub(super) fn collection_view(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let width = crate::navigation::content_width(window);
        let split = split_collection(width, self.section, self.selected.is_some());
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
                    .child(match &self.filter.artist {
                        // Coming from an artist, the page is named after them.
                        Some(artist) => self.page_title_with(
                            crate::i18n::Text::Albums,
                            self.artist_label(artist).into(),
                            cx,
                        ),
                        None => self.page_title(crate::i18n::Text::Albums, cx),
                    })
                    .child(self.measured_header(window, cx))
                    .child(self.albums_view(cx)),
            )
            .when(split, |view| {
                view.child(
                    div()
                        .w(rems(album_panel_width(width)))
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
