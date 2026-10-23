use super::{Facets, Home, HomeCatalog, HomeMetrics, fold};
use rap_core::model::{Album, Edition, LibrarySnapshot, Metadata, Track};
use std::sync::Arc;

fn album(id: i64, artist: &str, guests: &[&str], available: bool) -> Album {
    Album {
        id,
        title: format!("Album {id}"),
        artist: artist.into(),
        compilation: artist.is_empty(),
        added: 0,
        editions: vec![Edition {
            id,
            source_id: 1,
            folder: "source".into(),
            tracks: guests
                .iter()
                .enumerate()
                .map(|(i, name)| Track {
                    id: id * 100 + i as i64,
                    source_id: 1,
                    path: "song.flac".into(),
                    cover: None,
                    available,
                    metadata: Metadata {
                        artist: (*name).into(),
                        ..Default::default()
                    },
                })
                .collect(),
        }],
    }
}

fn snapshot() -> LibrarySnapshot {
    LibrarySnapshot {
        sources: vec![],
        albums: vec![
            album(10, "Alpha", &["Alpha", "Invité", "INVITÉ"], true),
            album(20, "Alpha", &["Alpha"], false),
            album(30, "", &["Invité", "Bêta"], true),
        ],
    }
}

#[test]
fn groups_album_and_track_artists_without_duplicate_albums_in_visible_order() {
    let snapshot = snapshot();
    let catalog = HomeCatalog::collect(&snapshot, &Facets::collect(&snapshot), &[2, 1, 0]);
    let albums = |name: &str| {
        &catalog
            .artists
            .iter()
            .find(|a| fold(&a.name) == fold(name))
            .unwrap()
            .albums
    };
    assert_eq!(albums("Alpha"), &[1, 0]);
    assert_eq!(albums("Invité"), &[2, 0]);
    assert_eq!(albums("Bêta"), &[2]);
    assert_eq!(catalog.artists.len(), 3);
}

#[test]
fn retains_offline_albums_and_only_lists_artists_with_matching_albums() {
    let snapshot = snapshot();
    let facets = Facets::collect(&snapshot);
    let catalog = HomeCatalog::collect(&snapshot, &facets, &[1]);
    assert_eq!(catalog.artists.len(), 1);
    assert_eq!(catalog.artists[0].name, "Alpha");
    assert_eq!(catalog.artists[0].albums, vec![1]);
    assert!(
        HomeCatalog::collect(&snapshot, &facets, &[])
            .artists
            .is_empty()
    );
}

#[test]
fn preserves_selection_by_id_and_replaces_it_when_artist_or_results_change() {
    let snapshot = snapshot();
    let facets = Facets::collect(&snapshot);
    let mut home = Home::new(Some("Alpha".into()));
    home.catalog = Arc::new(HomeCatalog::collect(&snapshot, &facets, &[1, 0, 2]));
    let mut selected = Some(10);
    assert!(!home.reconcile(&snapshot, &mut selected));
    home.artist = Some("Bêta".into());
    assert!(home.reconcile(&snapshot, &mut selected));
    assert_eq!(selected, Some(30));
    home.catalog = Arc::new(HomeCatalog::collect(&snapshot, &facets, &[1]));
    assert!(home.reconcile(&snapshot, &mut selected));
    assert_eq!(home.artist.as_deref(), Some("Alpha"));
    assert_eq!(selected, Some(20));
    home.catalog = Arc::default();
    assert!(home.reconcile(&snapshot, &mut selected));
    assert_eq!(home.artist, None);
    assert_eq!(selected, None);
}

#[test]
fn album_column_follows_selected_artist_including_guest_appearances_and_filters() {
    let mut snapshot = snapshot();
    snapshot.albums[0].title = "A very long album title ".repeat(8);
    let facets = Facets::collect(&snapshot);
    let metrics = Arc::new(HomeMetrics::measure(&snapshot, &facets));
    let mut catalog = HomeCatalog::collect(&snapshot, &facets, &[0, 1, 2]);
    catalog.set_metrics(&snapshot, metrics.clone());
    let mut home = Home::new(Some("Alpha".into()));
    home.catalog = Arc::new(catalog);
    let wide = home.album_column_width();
    home.artist = Some("Bêta".into());
    let narrow = home.album_column_width();
    assert!(wide > narrow);
    home.artist = home
        .catalog
        .artists
        .iter()
        .find(|a| fold(&a.name) == "invite")
        .map(|a| a.name.clone());
    assert_eq!(home.album_column_width(), wide);
    home.artist = Some("Alpha".into());
    assert_eq!(home.album_column_width(), wide);
    // The old metrics stay cached; excluding the long album must still shrink the pane.
    let mut filtered = HomeCatalog::collect(&snapshot, &facets, &[1, 2]);
    filtered.set_metrics(&snapshot, metrics);
    home.catalog = Arc::new(filtered);
    assert!(home.album_column_width() < wide);
    home.artist = None;
    assert_eq!(home.album_column_width(), 128.);
}
