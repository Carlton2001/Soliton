use super::moved;

#[test]
fn arrows_move_within_rows_and_columns() {
    assert_eq!(moved(4, 10, 3, "left"), Some(3));
    assert_eq!(moved(4, 10, 3, "right"), Some(5));
    assert_eq!(moved(4, 10, 3, "up"), Some(1));
    assert_eq!(moved(4, 10, 3, "down"), Some(7));
}

#[test]
fn arrows_stop_at_list_edges() {
    assert_eq!(moved(0, 10, 3, "left"), Some(0));
    assert_eq!(moved(1, 10, 3, "up"), Some(0));
    assert_eq!(moved(8, 10, 3, "down"), Some(9));
    assert_eq!(moved(9, 10, 3, "right"), Some(9));
}

#[test]
fn single_column_lists_step_by_one() {
    assert_eq!(moved(2, 5, 0, "down"), Some(3));
    assert_eq!(moved(2, 5, 1, "up"), Some(1));
}

#[test]
fn other_keys_and_empty_lists_do_not_move() {
    assert_eq!(moved(2, 5, 1, "enter"), None);
    assert_eq!(moved(0, 0, 1, "down"), None);
}
