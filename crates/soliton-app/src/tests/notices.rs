use super::{Level, Notices, Text};

#[test]
fn a_returning_source_clears_only_its_own_alert() {
    let mut notices = Notices::default();
    notices.source_problem(
        1,
        Level::Warning,
        Text::Warning,
        "First source offline".into(),
    );
    notices.source_problem(
        2,
        Level::Warning,
        Text::Warning,
        "Second source offline".into(),
    );
    notices.push(Level::Error, Text::Library, "Other library error");
    notices.source_problem(
        1,
        Level::Error,
        Text::Library,
        "First source timed out".into(),
    );

    assert_eq!(notices.items.len(), 3);
    assert!(notices.items[0].level == Level::Error);
    assert!(notices.items[0].label == Text::Library);
    assert_eq!(notices.items[0].text, "First source timed out");

    notices.resolve_source(1);

    assert_eq!(notices.items.len(), 2);
    assert_eq!(notices.items[0].source_id, Some(2));
    assert_eq!(notices.items[1].text, "Other library error");
}
