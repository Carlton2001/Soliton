use super::*;

const WORK: RECT = RECT {
    left: 1920,
    top: 0,
    right: 3840,
    bottom: 1040,
};

#[test]
fn opens_centered_above_the_pointer() {
    let anchor = TrayAnchor { x: 3000, y: 1000 };
    assert_eq!(origin(anchor, (200, 300), WORK), (2900, 700));
}

#[test]
fn stays_inside_the_work_area() {
    // Taskbar on the bottom, pointer in the corner.
    let corner = TrayAnchor { x: 3830, y: 1070 };
    assert_eq!(origin(corner, (200, 300), WORK), (3640, 740));
    // Taskbar on the top and left of a secondary monitor.
    let start = TrayAnchor { x: 1925, y: 5 };
    assert_eq!(origin(start, (200, 300), WORK), (1920, 0));
}
