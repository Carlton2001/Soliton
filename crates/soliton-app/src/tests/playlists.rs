use super::shared_album;
use soliton_core::model::{Metadata, Track};

fn track(album: &str) -> Track {
    Track {
        id: 1,
        source_id: 1,
        path: "a.flac".into(),
        metadata: Metadata {
            album: album.into(),
            ..Default::default()
        },
        cover: None,
        available: true,
    }
}

#[test]
fn only_tracks_of_one_named_album_suggest_a_name() {
    assert_eq!(shared_album(&[track("A"), track("A")]), Some("A"));
    assert_eq!(shared_album(&[track("A"), track("B")]), None);
    assert_eq!(shared_album(&[track(" ")]), None);
    assert_eq!(shared_album(&[]), None);
}
