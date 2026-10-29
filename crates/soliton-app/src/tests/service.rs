use super::{Event, Filter, Services, Work};
use soliton_core::playlist::PlaylistChange;
use soliton_library::{Cancellation, Library};
use soliton_platform::AppPaths;
use std::time::Duration;

#[test]
fn catalogue_loads_on_the_query_worker_without_any_window_or_ui_event_loop() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("music");
    let album = source.join("Nirvana/Bleach");
    std::fs::create_dir_all(&album).unwrap();
    // A generated fixture, never a file in the user's collection.
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(38_u32.to_le_bytes());
    bytes.extend(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    bytes.extend(44_100_u32.to_le_bytes());
    bytes.extend(88_200_u32.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data\x02\0\0\0\0\0");
    std::fs::write(album.join("01 - Blew.wav"), bytes).unwrap();
    let paths = AppPaths::at(directory.path().join("data"));
    let mut library = Library::open(&paths.root).unwrap();
    let source = library.add_source(&source).unwrap();
    let report = library
        .scan(source, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(report.progress.indexed, 1);
    drop(library);

    let services = Services::start(paths);
    for _ in 0..2 {
        services.query.send(Filter::default()).unwrap();
        let view = services
            .views
            .recv_timeout(Duration::from_secs(10))
            .expect("Query worker did not publish the catalogue")
            .unwrap();
        assert_eq!(view.visible.len(), 1);
        assert_eq!(view.tracks.len(), 1);
        assert_eq!(view.facets.artist_cards.len(), 1);
        assert_eq!(view.facets.artist_cards[0].name, "Nirvana");
        assert_eq!(view.facets.artist_cards[0].albums, 1);
    }
}

#[test]
fn playlist_edits_pass_a_running_scan_instead_of_waiting_for_its_end() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("music");
    let album = source.join("Artist/Album");
    std::fs::create_dir_all(&album).unwrap();
    // Generated one-frame PCM fixtures, never files of the user's collection.
    let mut bytes = b"RIFF".to_vec();
    bytes.extend(38_u32.to_le_bytes());
    bytes.extend(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    bytes.extend(44_100_u32.to_le_bytes());
    bytes.extend(88_200_u32.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data\x02\0\0\0\0\0");
    for index in 1..=8 {
        std::fs::write(album.join(format!("{index:02} - Track.wav")), &bytes).unwrap();
    }
    let services = Services::start(AppPaths::at(directory.path().join("data")));
    // Queued behind the scan: without interludes it waited for the whole scan.
    services
        .work
        .send(Work::Scan(vec![source], Cancellation::default()))
        .unwrap();
    services
        .work
        .send(Work::Playlist(PlaylistChange::Create {
            name: "Mix".into(),
            tracks: Vec::new(),
        }))
        .unwrap();
    let mut created_during_scan = false;
    loop {
        match services
            .events
            // Shared Windows runners have stalled more than 10 s between events.
            .recv_timeout(Duration::from_secs(60))
            .expect("Library worker did not finish the scan")
        {
            Event::Playlists(lists) if lists.len() == 1 => created_during_scan = true,
            Event::Finished(cancelled) => {
                assert!(!cancelled);
                break;
            }
            Event::Error(error) => panic!("{error}"),
            _ => {}
        }
    }
    assert!(created_during_scan);
}
