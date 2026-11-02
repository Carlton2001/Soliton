use super::{MAX_NAME_CHARS, same_name, unique_name};

#[test]
fn names_compare_without_case_or_surrounding_spaces() {
    assert!(same_name(" \u{c9}t\u{e9} ", "\u{e9}T\u{c9}"));
    assert!(!same_name("Rock", "Rock 2"));
}

#[test]
fn unique_name_numbers_only_taken_names() {
    let names = ["Rock", "rock (2)", "Jazz"];
    assert_eq!(unique_name("Blues", names.iter().copied()), "Blues");
    assert_eq!(unique_name("ROCK", names.iter().copied()), "ROCK (3)");
    assert_eq!(unique_name("Jazz", names.iter().copied()), "Jazz (2)");
}

#[test]
fn numbered_name_stays_within_the_limit() {
    let long = "x".repeat(MAX_NAME_CHARS);
    let names = [long.as_str()];
    let unique = unique_name(&long, names.iter().copied());
    assert_eq!(unique.chars().count(), MAX_NAME_CHARS);
    assert!(unique.ends_with(" (2)"));
}
