use super::{scroll_for_caret, single_line};
use gpui::px;

#[test]
fn pasted_line_breaks_and_tabs_become_single_spaces() {
    assert_eq!(single_line("a\r\nb\nc\rd\te"), "a b c d e");
    assert_eq!(single_line("plain"), "plain");
}

#[test]
fn the_line_scrolls_just_enough_to_keep_the_caret_visible() {
    let width = px(102.);
    // Text that fits never scrolls.
    assert_eq!(scroll_for_caret(px(0.), px(80.), px(80.), width), px(0.));
    // Typing past the right edge moves the line by the overflow only.
    assert_eq!(scroll_for_caret(px(0.), px(150.), px(150.), width), px(50.));
    // A caret inside the visible part keeps the current offset.
    assert_eq!(scroll_for_caret(px(50.), px(90.), px(150.), width), px(50.));
    // Moving left of the visible part brings the caret to the left edge.
    assert_eq!(scroll_for_caret(px(50.), px(20.), px(150.), width), px(20.));
    // Deleting text never leaves blank space after its end.
    assert_eq!(
        scroll_for_caret(px(50.), px(110.), px(110.), width),
        px(10.)
    );
}
