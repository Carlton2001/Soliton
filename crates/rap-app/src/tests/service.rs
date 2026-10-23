use super::{Filter, Services};
use rap_library::{Cancellation, Library};
use rap_platform::AppPaths;
use std::time::Duration;

#[test]
fn home_catalogue_loads_on_the_query_worker_without_any_window_or_ui_event_loop() {
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
            .expect("Query worker did not publish the Home catalogue")
            .unwrap();
        assert_eq!(view.visible.len(), 1);
        assert_eq!(view.tracks.len(), 1);
        assert_eq!(view.home.artists.len(), 1);
        assert_eq!(view.home.artists[0].name, "Nirvana");
        assert_eq!(view.home.artists[0].albums, vec![0]);
    }
}
