//! Button content: icon and label, or the icon alone when space runs short.
//! The choice is global, so buttons never show a partial or mixed state.
use gpui::{prelude::*, *};
use std::cell::Cell;

/// How many labels the current frame keeps.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Density {
    /// Every button shows its label.
    Full,
    /// Only the main actions keep theirs.
    Compact,
    /// Every button is reduced to its icon.
    Tight,
}

thread_local! {
    static DENSITY: Cell<Density> = const { Cell::new(Density::Full) };
}

/// Set once per frame, before any button is built.
pub fn set_density(density: Density) {
    DENSITY.with(|d| d.set(density));
}

pub(super) struct Content {
    pub icon: &'static str,
    pub label: SharedString,
    pub color: Rgba,
    /// Main actions, such as Play, are the last to lose their label.
    pub primary: bool,
}

impl IntoElement for Content {
    type Element = Div;
    fn into_element(self) -> Div {
        let show_label = !self.label.is_empty()
            && match DENSITY.with(Cell::get) {
                Density::Full => true,
                Density::Compact => self.primary,
                Density::Tight => false,
            };
        div()
            .flex()
            .items_center()
            .justify_center()
            .gap(rems(0.5))
            .whitespace_nowrap()
            .child(super::tinted_icon(self.icon, self.color))
            .when(show_label, |view| view.child(self.label))
    }
}
