use super::*;
use rap_core::model::{Album, Edition, Metadata, Track};

fn track(title: &str, artist: &str, genre: &str, year: u32, format: &str) -> Track {
    Track {
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
            format: format.into(),
            ..Default::default()
        },
    }
}

fn album(title: &str, artist: &str, editions: Vec<Vec<Track>>) -> Album {
    Album {
        id: 1,
        title: title.into(),
        artist: artist.into(),
        compilation: artist.is_empty(),
        editions: editions
            .into_iter()
            .enumerate()
            .map(|(id, tracks)| Edition {
                id: id as i64,
                source_id: 1,
                folder: "folder".into(),
                tracks,
            })
            .collect(),
    }
}

#[test]
fn searches_track_artists_and_combines_filters_on_the_same_track() {
    let catalog = Catalog::new(LibrarySnapshot {
        sources: vec![],
        albums: vec![album(
            "Compilation",
            "Collectif",
            vec![vec![
                track("Été", "Invité", "Jazz", 2020, "FLAC"),
                track("Hiver", "Autre", "Rock", 2022, "FLAC"),
            ]],
        )],
    });
    let mut filter = Filter {
        query: "invité".into(),
        ..Default::default()
    };
    assert_eq!(catalog.matches(&filter), vec![0]);
    filter.genre = Some("Rock".into());
    assert!(catalog.matches(&filter).is_empty());
    filter.query.clear();
    filter.year = Some("2020".into());
    assert!(catalog.matches(&filter).is_empty());
    filter.year = Some("2022".into());
    assert_eq!(catalog.matches(&filter), vec![0]);
}

#[test]
fn searches_every_edition_and_labels_compilations() {
    let catalog = Catalog::new(LibrarySnapshot {
        sources: vec![],
        albums: vec![
            album(
                "Kind of Blue",
                "Miles Davis",
                vec![
                    vec![track("So What", "Miles Davis", "Jazz", 1959, "FLAC")],
                    vec![track("So What", "Miles Davis", "Jazz", 1959, "MP3")],
                ],
            ),
            album(
                "Hits",
                "",
                vec![vec![track("Song", "Alpha", "Pop", 2001, "MP3")]],
            ),
        ],
    });
    let filter = Filter {
        format: Some("MP3".into()),
        ..Default::default()
    };
    assert_eq!(catalog.matches(&filter), vec![0, 1]);
    for query in ["artistes divers", "VARIOUS"] {
        let filter = Filter {
            query: query.into(),
            ..Default::default()
        };
        assert_eq!(catalog.matches(&filter), vec![1]);
    }
    assert_eq!(catalog.facets.artists, ["Alpha", "Miles Davis"]);
}
