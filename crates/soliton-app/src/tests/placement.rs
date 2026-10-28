use super::{MIN_SIZE, restored};
use gpui::{Bounds, point, px, size};
use soliton_core::config::WindowPlacement;

fn placement(x: f32, y: f32, maximized: bool) -> WindowPlacement {
    WindowPlacement {
        x,
        y,
        width: 900.,
        height: 700.,
        maximized,
    }
}

#[test]
fn saved_bounds_follow_the_connected_screens() {
    let screen = Bounds::new(point(px(0.), px(0.)), size(px(1920.), px(1080.)));
    let expected = Bounds::new(point(px(100.), px(50.)), size(px(900.), px(700.)));

    assert_eq!(
        restored(Some(placement(100., 50., false)), &[screen]),
        Some((expected, false))
    );
    assert_eq!(
        restored(Some(placement(100., 50., true)), &[screen]),
        Some((expected, true))
    );
    // Title bar on a disconnected screen.
    assert_eq!(
        restored(Some(placement(3000., 50., false)), &[screen]),
        None
    );
    assert_eq!(restored(None, &[screen]), None);
}

#[test]
fn saved_bounds_are_kept_before_screens_are_known() {
    let expected = Bounds::new(point(px(0.), px(0.)), size(px(900.), px(700.)));
    assert_eq!(
        restored(Some(placement(0., 0., true)), &[]),
        Some((expected, true))
    );

    let mut small = placement(0., 0., false);
    small.width = 100.;
    small.height = 100.;
    assert_eq!(
        restored(Some(small), &[]),
        Some((Bounds::new(point(px(0.), px(0.)), MIN_SIZE), false))
    );
}
