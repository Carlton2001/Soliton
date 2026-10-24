use super::{Section, split_collection};

#[test]
fn album_panel_requires_room_for_both_the_collection_and_record() {
    assert!(!split_collection(71.99, Section::Albums, true));
    assert!(split_collection(72., Section::Albums, true));
    assert!(split_collection(90., Section::Albums, true));
}

#[test]
fn other_pages_and_empty_selection_keep_their_existing_layout() {
    assert!(!split_collection(90., Section::Albums, false));
    for section in [
        Section::Artists,
        Section::Queue,
        Section::Playlists,
        Section::Settings,
    ] {
        assert!(!split_collection(90., section, true));
    }
}
