use super::digit;

#[test]
fn digits_are_read_on_qwerty_azerty_and_keypad() {
    assert_eq!(digit("1"), Some(1));
    assert_eq!(digit("0"), Some(0));
    assert_eq!(digit("&"), Some(1));
    assert_eq!(digit("é"), Some(2));
    assert_eq!(digit("\""), Some(3));
    assert_eq!(digit("("), Some(5));
    assert_eq!(digit("à"), Some(0));
    assert_eq!(digit("12"), None);
    assert_eq!(digit("a"), None);
}
