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

#[test]
fn case_and_spacing_variants_share_one_facet_and_filter() {
    let mut first = album(
        "Back in Black",
        "AC/DC",
        vec![vec![
            track("Hells Bells", "AC/DC", "Rock", 1980, "FLAC"),
            track("Shoot to Thrill", "AC/DC", "rock", 1980, "FLAC"),
        ]],
    );
    first.id = 1;
    let second = album(
        "Highway to Hell",
        "Ac/Dc",
        vec![vec![track(
            "Touch Too Much",
            "ac/dc",
            "ROCK ",
            1979,
            "FLAC",
        )]],
    );
    let third = album(
        "Arrival",
        "abba",
        vec![vec![track("Dancing Queen", "abba", "Pop", 1976, "FLAC")]],
    );
    let fourth = album(
        "Stripes",
        "Zebra",
        vec![vec![track("Stripes", "Zebra", "Pop", 1990, "FLAC")]],
    );
    let catalog = Catalog::new(LibrarySnapshot {
        sources: vec![],
        albums: vec![first, second, third, fourth],
    });
    // The most frequent spelling is shown, the first met on ties; order ignores case.
    assert_eq!(catalog.facets.artists, ["abba", "AC/DC", "Zebra"]);
    assert_eq!(catalog.facets.genres, ["Pop", "Rock"]);
    let filter = Filter {
        artist: Some("AC/DC".into()),
        genre: Some("Rock".into()),
        ..Default::default()
    };
    assert_eq!(catalog.matches(&filter), vec![0, 1]);
    let filter = Filter {
        query: "  HIGHWAY   to ".into(),
        ..Default::default()
    };
    assert_eq!(catalog.matches(&filter), vec![1]);
}
