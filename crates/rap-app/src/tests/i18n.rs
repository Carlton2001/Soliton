use super::*;

#[test]
fn failures_read_in_each_language_and_keep_their_detail() {
    let unavailable = Failure::with(Kind::OutputUnavailable, "Speakers (USB)");
    assert_eq!(
        describe(Language::French, &unavailable),
        "Sortie audio indisponible : Speakers (USB)"
    );
    assert_eq!(
        describe(Language::English, &unavailable),
        "Audio output unavailable: Speakers (USB)"
    );
    assert_eq!(
        describe(Language::French, &Failure::new(Kind::Busy)),
        "RAP est occupé, réessayez"
    );
    // Unclassified messages, such as a receiver's own error, are shown as is.
    let other = Failure::with(Kind::Other, "Receiver error 714");
    assert_eq!(describe(Language::French, &other), "Receiver error 714");
}
