use super::keyboard_value;

#[test]
fn keyboard_sliders_reach_the_bounds_without_wrapping() {
    assert_eq!(keyboard_value("left", 0.01, 0.05), Some(0.));
    assert_eq!(keyboard_value("up", 0.99, 0.05), Some(1.));
    assert_eq!(keyboard_value("home", 0.6, 0.05), Some(0.));
    assert_eq!(keyboard_value("end", 0.6, 0.05), Some(1.));
    assert_eq!(keyboard_value("enter", 0.6, 0.05), None);
}
