use super::*;
use crate::Cancellation;

fn wav(path: &Path) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, b"RIFF\x26\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x44\xac\0\0\x88\x58\x01\0\x02\0\x10\0data\x02\0\0\0\0\0").unwrap();
}

#[test]
fn indexes_incrementally_persists_and_preserves_disconnected_sources() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    let audio = source.join("Artist/Album/01 - Song.wav");
    wav(&audio);
    let before = std::fs::read(&audio).unwrap();
    let storage = dir.path().join("data");
    let mut library = Library::open(&storage).unwrap();
    let id = library.add_source(&source).unwrap();
    let report = library.scan(id, &Cancellation::default(), |_| {}).unwrap();
    assert!(report.complete, "{:?}", report.diagnostics);
    assert_eq!(report.progress.indexed, 1);
    let report = library.scan(id, &Cancellation::default(), |_| {}).unwrap();
    assert_eq!(report.progress.unchanged, 1);
    assert_eq!(report.progress.indexed, 0);
    assert_eq!(std::fs::read(&audio).unwrap(), before);
    drop(library);
    let mut library = Library::open(&storage).unwrap();
    assert_eq!(library.snapshot().unwrap().albums.len(), 1);
    std::fs::rename(&source, dir.path().join("offline")).unwrap();
    assert!(library.scan(id, &Cancellation::default(), |_| {}).is_err());
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.albums.len(), 1);
    assert!(!snapshot.albums[0].tracks[0].available);
    std::fs::rename(dir.path().join("offline"), &source).unwrap();
    library.scan(id, &Cancellation::default(), |_| {}).unwrap();
    assert!(library.snapshot().unwrap().albums[0].tracks[0].available);
    std::fs::remove_file(&audio).unwrap();
    library.scan(id, &Cancellation::default(), |_| {}).unwrap();
    assert!(!library.snapshot().unwrap().albums[0].tracks[0].available);
}

#[test]
fn corruption_and_cancellation_never_establish_disappearance() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    let audio = source.join("Artist/Album/01.wav");
    wav(&audio);
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    library.scan(id, &Cancellation::default(), |_| {}).unwrap();
    std::fs::remove_file(&audio).unwrap();
    std::fs::write(source.join("broken.flac"), b"invalid").unwrap();
    let report = library.scan(id, &Cancellation::default(), |_| {}).unwrap();
    assert_eq!(report.progress.errors, 1);
    assert!(!report.complete);
    assert!(library.snapshot().unwrap().albums[0].tracks[0].available);
    let cancellation = Cancellation::default();
    cancellation.cancel();
    let report = library.scan(id, &cancellation, |_| {}).unwrap();
    assert!(report.cancelled);
    assert!(library.snapshot().unwrap().albums[0].tracks[0].available);
}

#[test]
fn rejects_storage_overlap_and_keeps_editions_separate() {
    let dir = tempfile::tempdir().unwrap();
    let storage = dir.path().join("data");
    let mut library = Library::open(&storage).unwrap();
    assert!(library.add_source(dir.path()).is_err());
    assert!(library.add_source(&storage).is_err());
    let source = dir.path().join("music");
    wav(&source.join("Artist/Album16/01.wav"));
    wav(&source.join("Artist/Album24/01.wav"));
    let id = library.add_source(&source).unwrap();
    assert!(library.add_source(&source.join("Artist")).is_err());
    library.scan(id, &Cancellation::default(), |_| {}).unwrap();
    assert_eq!(library.snapshot().unwrap().albums.len(), 2);
}

#[test]
fn bounded_pipeline_processes_more_than_all_queue_capacities() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("music");
    for index in 0..180 {
        wav(&source.join(format!("Artist/Album/{index:03}.wav")));
    }
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&source).unwrap();
    let report = library.scan(id, &Cancellation::default(), |_| {}).unwrap();
    assert_eq!(report.progress.indexed, 180);
    assert!(report.complete);
}

#[test]
fn removing_a_source_forgets_only_its_entries_and_keeps_files() {
    let dir = tempfile::tempdir().unwrap();
    let kept = dir.path().join("kept");
    let removed = dir.path().join("removed");
    wav(&kept.join("Artist/Kept/01 - Song.wav"));
    let audio = removed.join("Other/Gone/01 - Song.wav");
    wav(&audio);
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let kept_id = library.add_source(&kept).unwrap();
    let removed_id = library.add_source(&removed).unwrap();
    for id in [kept_id, removed_id] {
        library.scan(id, &Cancellation::default(), |_| {}).unwrap();
    }
    assert_eq!(library.snapshot().unwrap().albums.len(), 2);
    library.remove_source(removed_id).unwrap();
    let snapshot = library.snapshot().unwrap();
    assert_eq!(snapshot.sources.len(), 1);
    assert_eq!(snapshot.sources[0].id, kept_id);
    assert_eq!(snapshot.albums.len(), 1);
    assert_eq!(snapshot.albums[0].tracks[0].source_id, kept_id);
    assert!(audio.exists());
    // The folder can be added and indexed again afterwards.
    let again = library.add_source(&removed).unwrap();
    library
        .scan(again, &Cancellation::default(), |_| {})
        .unwrap();
    assert_eq!(library.snapshot().unwrap().albums.len(), 2);
}
