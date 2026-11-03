use super::{moved, next_with_letter};

#[test]
fn arrows_move_within_rows_and_columns() {
    assert_eq!(moved(4, 10, 3, 1, "left"), Some(3));
    assert_eq!(moved(4, 10, 3, 1, "right"), Some(5));
    assert_eq!(moved(4, 10, 3, 1, "up"), Some(1));
    assert_eq!(moved(4, 10, 3, 1, "down"), Some(7));
}

#[test]
fn arrows_stop_at_list_edges() {
    assert_eq!(moved(0, 10, 3, 1, "left"), Some(0));
    assert_eq!(moved(1, 10, 3, 1, "up"), Some(0));
    assert_eq!(moved(8, 10, 3, 1, "down"), Some(9));
    assert_eq!(moved(9, 10, 3, 1, "right"), Some(9));
}

#[test]
fn single_column_lists_step_by_one() {
    assert_eq!(moved(2, 5, 0, 1, "down"), Some(3));
    assert_eq!(moved(2, 5, 1, 1, "up"), Some(1));
}

#[test]
fn other_keys_and_empty_lists_do_not_move() {
    assert_eq!(moved(2, 5, 1, 1, "enter"), None);
    assert_eq!(moved(0, 0, 1, 1, "down"), None);
}

#[test]
fn home_end_and_pages_jump_by_visible_rows() {
    assert_eq!(moved(4, 30, 3, 2, "pagedown"), Some(10));
    assert_eq!(moved(10, 30, 3, 2, "pageup"), Some(4));
    assert_eq!(moved(28, 30, 3, 2, "pagedown"), Some(29));
    assert_eq!(moved(7, 30, 3, 2, "home"), Some(0));
    assert_eq!(moved(7, 30, 3, 2, "end"), Some(29));
}

#[test]
fn letters_cycle_through_matching_names() {
    let names = ["Abba", "Bach", "Björk", "Beck"];
    assert_eq!(next_with_letter(names.iter().copied(), None, 'b'), Some(1));
    assert_eq!(
        next_with_letter(names.iter().copied(), Some(1), 'b'),
        Some(2)
    );
    assert_eq!(
        next_with_letter(names.iter().copied(), Some(3), 'b'),
        Some(1)
    );
    assert_eq!(next_with_letter(names.iter().copied(), Some(0), 'z'), None);
}
