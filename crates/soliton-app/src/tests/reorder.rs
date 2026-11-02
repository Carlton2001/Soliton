#[test]
fn original_order_regroups_albums_by_first_appearance() {
    let entries = [
        (1, Some((7, 2))),
        (2, Some((3, 1))),
        (3, None),
        (4, Some((7, 0))),
        (5, Some((3, 0))),
    ];
    assert_eq!(super::original_order(&entries), [4, 1, 5, 2, 3]);
}

use super::{plan_move, preview, range_between, run_edges, start_at, toggled};

#[test]
fn ctrl_adds_and_removes_single_rows() {
    assert_eq!(toggled(&[1, 3], 2), [1, 3, 2]);
    assert_eq!(toggled(&[1, 3], 1), [3]);
}

#[test]
fn adjacent_moved_rows_share_one_run() {
    let rows = [1, 2, 3, 4, 5];
    let moved = [2, 3, 5];
    assert_eq!(run_edges(&rows, &moved, 1), (true, false));
    assert_eq!(run_edges(&rows, &moved, 2), (false, true));
    assert_eq!(run_edges(&rows, &moved, 4), (true, true));
}

#[test]
fn range_follows_list_order_in_both_directions() {
    let rows = [1, 2, 3, 4];
    assert_eq!(range_between(&rows, 2, 4), [2, 3, 4]);
    assert_eq!(range_between(&rows, 4, 2), [2, 3, 4]);
    assert_eq!(range_between(&rows, 9, 2), [2]);
}

#[test]
fn preview_gathers_the_block_among_the_other_rows() {
    let rows = [1, 2, 3, 4, 5];
    assert_eq!(preview(&rows, &[2, 4], 0), [2, 4, 1, 3, 5]);
    assert_eq!(preview(&rows, &[2, 4], 2), [1, 3, 2, 4, 5]);
    assert_eq!(preview(&rows, &[2, 4], 9), [1, 3, 5, 2, 4]);
}

#[test]
fn plan_targets_the_row_following_the_block() {
    let rows = [1, 2, 3, 4, 5];
    assert_eq!(plan_move(&rows, &[4], 1), Some(Some(2)));
    assert_eq!(plan_move(&rows, &[1, 3], 3), Some(None));
    assert_eq!(plan_move(&rows, &[1, 3], 2), Some(Some(5)));
}

#[test]
fn drop_that_keeps_the_order_is_ignored() {
    let rows = [1, 2, 3, 4];
    assert_eq!(plan_move(&rows, &[2], 1), None);
    assert_eq!(plan_move(&rows, &[2, 3], 1), None);
    assert_eq!(plan_move(&rows, &[4], 3), None);
}

#[test]
fn start_keeps_the_grabbed_row_under_the_pointer() {
    assert_eq!(start_at(25., 0., 10., 0, 3), 2);
    assert_eq!(start_at(25., 0., 10., 1, 3), 1);
    assert_eq!(start_at(5., 0., 10., 1, 3), 0);
    assert_eq!(start_at(5., -20., 10., 0, 3), 2);
    assert_eq!(start_at(500., 0., 10., 0, 3), 3);
}
