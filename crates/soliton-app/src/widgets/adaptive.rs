//! Button content: icon and label, or the icon alone when space runs short.
//! The choice is global, so buttons never show a partial or mixed state.
//! Labels go only when the toolbar of the page cannot hold them.
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

/// Builds with another density, for a row that cannot hold its labels at
/// the window's minimum width; the page density is restored afterwards.
pub fn with_density<R>(density: Density, build: impl FnOnce() -> R) -> R {
    let page = DENSITY.with(|d| d.replace(density));
    let built = build();
    DENSITY.with(|d| d.set(page));
    built
}

/// Width in rems of a button label at body size, as drawn.
pub fn label_width(text: &str, window: &Window) -> f32 {
    let rem = window.rem_size();
    let run = TextRun {
        len: text.len(),
        font: Font {
            weight: FontWeight::MEDIUM,
            ..font(super::UI_FONT)
        },
        color: Hsla::default(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let line = window.text_system().shape_line(
        SharedString::from(text.to_owned()),
        rem * super::TEXT_BODY.0,
        &[run],
        None,
    );
    f32::from(line.width) / f32::from(rem)
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
