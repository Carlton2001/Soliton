//! Letter index beside the artist list, jumping to the first artist of a letter.
use crate::{app::Soliton, widgets::TEXT_CAPTION};
use gpui::{prelude::*, *};
use soliton_core::text::fold;

/// Space on each side of the letters, from the tiles and from the scrollbar, in rems.
const MARGIN: f32 = 0.5;
/// Width of the letter column itself, in rems.
const LETTERS_WIDTH: f32 = 0.75;
/// Lane reserved right of the artist list, in rems. The list already ends with
/// its own padding, which counts toward the left margin.
pub const LANE: f32 = MARGIN - crate::views::CONTENT_PADDING + LETTERS_WIDTH + MARGIN;

/// Names not starting with a Latin letter, digits included, gather under `#`.
pub const LETTERS: [char; 27] = [
    '#', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
    'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

/// Index letter of a name, accents and case ignored.
pub fn letter(name: &str) -> char {
    fold(name)
        .chars()
        .next()
        .filter(char::is_ascii_lowercase)
        .map_or('#', |c| c.to_ascii_uppercase())
}

/// Position of the first name filed under `target` in a list sorted by folded
/// name, else of the next filed letter, else of the last name. `#` has no
/// following letter: without such names the list stays where it is.
pub fn position<'a>(names: impl IntoIterator<Item = &'a str>, target: char) -> Option<usize> {
    let mut last = None;
    for (index, name) in names.into_iter().enumerate() {
        let found = letter(name);
        if found == target || (target != '#' && found != '#' && found > target) {
            return Some(index);
        }
        last = Some(index);
    }
    last.filter(|_| target != '#')
}

impl Soliton {
    /// Vertical strip in a lane right of the artist list. Clicking a letter,
    /// or dragging over the strip, scrolls to its first artist.
    pub fn alphabet_index(&self, names: Vec<String>, columns: usize, cx: &Context<Self>) -> Div {
        let theme = self.theme();
        let present: Vec<char> = names.iter().map(|name| letter(name)).collect();
        let shown = self.shown_artists(names.len(), columns);
        let in_view: Vec<char> = present.get(shown).unwrap_or_default().to_vec();
        // The displayed names, which a search may narrow.
        let names: std::rc::Rc<[String]> = names.into();
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .right(rems(MARGIN))
            .w(rems(LETTERS_WIDTH))
            .flex()
            .flex_col()
            // Spread over the whole height of the list.
            .justify_between()
            .text_size(TEXT_CAPTION)
            // Only letters with artists are offered.
            .children(
                LETTERS
                    .into_iter()
                    .enumerate()
                    .filter(|(_, target)| present.contains(target))
                    .map(|(index, target)| {
                        // Letters of the artists on screen stand out.
                        let mut color = theme.muted;
                        color.a = if in_view.contains(&target) { 1. } else { 0.5 };
                        let names = names.clone();
                        let jump = move |this: &mut Soliton, cx: &mut Context<Soliton>| {
                            cx.notify();
                            if let Some(position) =
                                position(names.iter().map(String::as_str), target)
                            {
                                this.facet_scroll
                                    .scroll_to_item(position / columns.max(1), ScrollStrategy::Top);
                            }
                        };
                        div()
                            .id(("letter", index))
                            .flex()
                            .justify_center()
                            .min_h_0()
                            .overflow_hidden()
                            .text_color(color)
                            .hover(move |s| s.text_color(theme.text))
                            .child(target.to_string())
                            .on_mouse_down(MouseButton::Left, {
                                let jump = jump.clone();
                                cx.listener(move |this, _, _, cx| jump(this, cx))
                            })
                            .on_mouse_move(cx.listener(
                                move |this, event: &MouseMoveEvent, _, cx| {
                                    if event.pressed_button == Some(MouseButton::Left) {
                                        jump(this, cx)
                                    }
                                },
                            ))
                    }),
            )
    }

    /// Artists at least partly visible, from the list geometry of the last frame.
    fn shown_artists(&self, count: usize, columns: usize) -> std::ops::Range<usize> {
        let state = self.facet_scroll.0.borrow();
        let rows = count.div_ceil(columns.max(1));
        let Some(size) = state.last_item_size.filter(|_| rows > 0) else {
            return 0..0;
        };
        let row = f32::from(size.contents.height) / rows as f32;
        if row <= 0. {
            return 0..0;
        }
        let top = -f32::from(state.base_handle.offset().y);
        let bottom = top + f32::from(size.item.height);
        let first = (top / row).floor().max(0.) as usize;
        let last = (bottom / row).ceil() as usize;
        (first * columns).min(count)..(last * columns).min(count)
    }
}

#[cfg(test)]
#[path = "tests/alphabet.rs"]
mod tests;
