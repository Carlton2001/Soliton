use super::thumb_geometry;

#[test]
fn thumb_reaches_both_ends_and_clamps_overscroll() {
    assert_eq!(thumb_geometry(100., 300., 0.), Some((0., 32.)));
    assert_eq!(thumb_geometry(100., 300., -150.), Some((34., 32.)));
    assert_eq!(thumb_geometry(100., 300., -300.), Some((68., 32.)));
    assert_eq!(thumb_geometry(100., 300., -900.), Some((68., 32.)));
    assert_eq!(thumb_geometry(100., 300., 100.), Some((0., 32.)));
}

#[test]
fn no_rail_for_content_that_fits_and_no_thumb_larger_than_its_rail() {
    assert_eq!(thumb_geometry(100., 0., 0.), None);
    assert_eq!(thumb_geometry(0., 100., 0.), None);
    assert_eq!(thumb_geometry(20., 100., -100.), Some((0., 20.)));
}
