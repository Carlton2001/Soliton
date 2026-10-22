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
        added: 0,
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
    filter.period = Some(Period::Year(2020));
    assert!(catalog.matches(&filter).is_empty());
    filter.period = Some(Period::Year(2022));
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

#[test]
fn periods_qualities_sorting_and_unavailable_albums() {
    let hires = |title: &str, year: u32, bits: u8, rate: u32| {
        let mut track = track(title, "Artist", "Jazz", year, "FLAC");
        track.metadata.bit_depth = Some(bits);
        track.metadata.sample_rate = Some(rate);
        track
    };
    let mut first = album("Zeta", "Artist", vec![vec![hires("a", 1975, 24, 96_000)]]);
    first.added = 10;
    let mut second = album("Alpha", "Artist", vec![vec![hires("b", 1979, 16, 44_100)]]);
    second.added = 30;
    let mut third = album(
        "Mid",
        "Artist",
        vec![vec![track("c", "Artist", "Jazz", 1981, "MP3")]],
    );
    third.added = 20;
    third.editions[0].tracks[0].available = false;
    let catalog = Catalog::new(LibrarySnapshot {
        sources: vec![],
        albums: vec![first, second, third],
    });
    let all = catalog.options(&Filter::default());
    assert_eq!(
        all.years.into_iter().collect::<Vec<_>>(),
        [1975, 1979, 1981]
    );
    // Each filter only offers values reachable under the others.
    let options = catalog.options(&Filter {
        format: Some("MP3".into()),
        ..Default::default()
    });
    assert_eq!(options.years.iter().copied().collect::<Vec<_>>(), [1981]);
    assert_eq!(options.formats.len(), 2);
    assert_eq!(
        options.qualities.into_iter().collect::<Vec<_>>(),
        [Quality::Lossy]
    );
    let mut filter = Filter {
        period: Some(Period::Decade(1970)),
        ..Default::default()
    };
    assert_eq!(catalog.matches(&filter), vec![0, 1]);
    filter.quality = Some(Quality::HiRes);
    assert_eq!(catalog.matches(&filter), vec![0]);
    filter.quality = Some(Quality::Cd);
    assert_eq!(catalog.matches(&filter), vec![1]);
    let mut filter = Filter {
        quality: Some(Quality::Lossy),
        ..Default::default()
    };
    assert_eq!(catalog.matches(&filter), vec![2]);
    filter.hide_unavailable = true;
    assert!(catalog.matches(&filter).is_empty());
    let mut filter = Filter::default();
    for (sort, order) in [
        (AlbumSort::Artist, vec![0, 1, 2]),
        (AlbumSort::Title, vec![1, 2, 0]),
        (AlbumSort::Year, vec![0, 1, 2]),
        (AlbumSort::Added, vec![1, 2, 0]),
    ] {
        filter.sort = sort;
        assert_eq!(catalog.matches(&filter), order, "{sort:?}");
    }
}

#[test]
fn artist_cover_comes_from_the_latest_credited_album() {
    let mut late = album(
        "Late",
        "Band",
        vec![vec![track("A", "Band", "Rock", 1999, "FLAC")]],
    );
    late.editions[0].tracks[0].cover = Some("late.png".into());
    let mut early = album(
        "Early",
        "Band",
        vec![vec![track("B", "Band", "Rock", 1971, "FLAC")]],
    );
    early.editions[0].tracks[0].cover = Some("early.png".into());
    // A newer album where the band only guests does not represent it.
    let mut guest = album(
        "Guest",
        "Other",
        vec![vec![track("C", "Band", "Rock", 2020, "FLAC")]],
    );
    guest.editions[0].tracks[0].cover = Some("guest.png".into());
    let facets = Facets::collect(&LibrarySnapshot {
        sources: vec![],
        albums: vec![late, guest, early],
    });
    let band = facets
        .artist_cards
        .iter()
        .find(|card| card.name == "Band")
        .unwrap();
    assert_eq!(band.albums, 3);
    assert_eq!(band.cover, Some("late.png".into()));
    let other = facets
        .artist_cards
        .iter()
        .find(|card| card.name == "Other")
        .unwrap();
    assert_eq!(
        (other.albums, other.cover.clone()),
        (1, Some("guest.png".into()))
    );
    assert_eq!(facets.artist_cards.len(), facets.artists.len());
}
