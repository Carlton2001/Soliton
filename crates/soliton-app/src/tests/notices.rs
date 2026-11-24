use super::{Level, Notices, Text};

#[test]
fn a_returning_source_clears_only_its_own_alert() {
    let mut notices = Notices::default();
    notices.source_unavailable(1, "First source offline".into());
    notices.source_unavailable(2, "Second source offline".into());
    notices.push(Level::Error, Text::Library, "Other library error");
    notices.source_unavailable(1, "First source still offline".into());

    notices.resolve_source(1);

    assert_eq!(notices.items.len(), 2);
    assert_eq!(notices.items[0].source_id, Some(2));
    assert_eq!(notices.items[1].text, "Other library error");
}
