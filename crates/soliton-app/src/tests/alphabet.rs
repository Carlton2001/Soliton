use super::{letter, position};

#[test]
fn letters_ignore_accents_and_gather_the_rest() {
    assert_eq!(letter("Émilie Simon"), 'E');
    assert_eq!(letter("björk"), 'B');
    assert_eq!(letter("10cc"), '#');
    assert_eq!(letter("坂本龍一"), '#');
    assert_eq!(letter(""), '#');
}

#[test]
fn position_falls_back_on_the_next_letter() {
    let names = ["10cc", "Air", "Björk", "Daft Punk", "Zazie"];
    assert_eq!(position(names, '#'), Some(0));
    assert_eq!(position(names, 'B'), Some(2));
    assert_eq!(position(names, 'C'), Some(3));
    assert_eq!(position(names, 'Y'), Some(4));
    assert_eq!(position(["Air"], 'Z'), Some(0));
    assert_eq!(position([], 'A'), None);
    // Without `#` names the list does not jump to its end.
    assert_eq!(position(["Air", "Zazie"], '#'), None);
}
