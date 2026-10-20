use super::*;
use rap_core::model::{Album, Metadata, Track};

#[test]
fn searches_track_artists_and_combines_filters_on_the_same_track() {
    let track = |title: &str, artist: &str, genre: &str, year: u32| Track {
        id: 1,
        source_id: 1,
        path: "track.flac".into(),
        cover: None,
        available: true,
        metadata: Metadata {
            title: title.into(),
            artist: artist.into(),
            genre: genre.into(),
            year: Some(year),
            format: "FLAC".into(),
            ..Default::default()
        },
    };
    let snapshot = LibrarySnapshot {
        sources: vec![],
        albums: vec![Album {
            id: 1,
            title: "Compilation".into(),
            artist: "Collectif".into(),
            tracks: vec![
                track("Été", "Invité", "Jazz", 2020),
                track("Hiver", "Autre", "Rock", 2022),
            ],
        }],
    };
    let mut filter = Filter {
        query: "invité".into(),
        ..Default::default()
    };
    assert_eq!(matches(&snapshot, &filter), vec![0]);
    filter.genre = Some("Rock".into());
    assert!(matches(&snapshot, &filter).is_empty());
    filter.query.clear();
    filter.year = Some("2020".into());
    assert!(matches(&snapshot, &filter).is_empty());
    filter.year = Some("2022".into());
    assert_eq!(matches(&snapshot, &filter), vec![0]);
}
