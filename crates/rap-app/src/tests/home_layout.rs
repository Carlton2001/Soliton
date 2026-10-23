use super::HomeMetrics;
use std::collections::HashMap;

#[test]
fn fixed_columns_follow_content_width_without_manual_overrides() {
    let mut metrics = HomeMetrics {
        artists: 410.,
        albums: HashMap::from([(1, 620.), (2, 220.)]),
        tracks: HashMap::new(),
    };
    assert_eq!(metrics.artists_width(), 410.);
    assert_eq!(metrics.album_width(1), 620.);
    assert_eq!(metrics.album_width(2), 220.);
    metrics.artists = 510.;
    metrics.albums.insert(1, 720.);
    assert_eq!(metrics.artists_width(), 510.);
    assert_eq!(metrics.album_width(1), 720.);
    assert_eq!(metrics.album_width(2), 220.);
}

#[test]
fn horizontal_track_content_uses_the_selected_editions_measured_width() {
    let metrics = HomeMetrics {
        tracks: HashMap::from([(1, 820.), (2, 400.)]),
        ..Default::default()
    };
    assert_eq!(metrics.tracks_width(1), 820.);
    assert_eq!(metrics.tracks_width(2), 512.);
    assert_eq!(metrics.tracks_width(3), 512.);
}

#[test]
fn independent_workers_measure_a_large_library_without_a_window_cache() {
    use crate::catalog::Facets;
    use rap_core::model::{Album, Edition, LibrarySnapshot, Metadata, Track};
    use std::{
        sync::{Arc, mpsc},
        time::Duration,
    };
    let snapshot = Arc::new(LibrarySnapshot {
        sources: vec![],
        albums: vec![Album {
            id: 1,
            title: "Album with a long title — Édition complète".into(),
            artist: "Artist with a long name".into(),
            compilation: false,
            added: 0,
            editions: vec![Edition {
                id: 1,
                source_id: 1,
                folder: "fixture".into(),
                tracks: (0..1000)
                    .map(|index| Track {
                        id: index,
                        source_id: 1,
                        path: format!("{index}.flac").into(),
                        available: true,
                        cover: None,
                        metadata: Metadata {
                            title: format!("Long track title {index} — Été"),
                            artist: "Artist with a long name".into(),
                            ..Default::default()
                        },
                    })
                    .collect(),
            }],
        }],
    });
    let facets = Arc::new(Facets::collect(&snapshot));
    let (send, receive) = mpsc::channel();
    for _ in 0..2 {
        let (snapshot, facets, send) = (snapshot.clone(), facets.clone(), send.clone());
        std::thread::spawn(move || {
            let metrics = HomeMetrics::measure(&snapshot, &facets);
            send.send((
                [metrics.artists_width(), metrics.album_width(1)],
                metrics.tracks_width(1),
            ))
            .unwrap();
        });
    }
    let first = receive
        .recv_timeout(Duration::from_secs(10))
        .expect("Text measurement blocked");
    let second = receive
        .recv_timeout(Duration::from_secs(10))
        .expect("Concurrent text measurement blocked");
    assert_eq!(first, second);
    assert!(first.0[0] > 96. && first.0[1] > 128.);
    assert!(first.1 >= 512.);
}
