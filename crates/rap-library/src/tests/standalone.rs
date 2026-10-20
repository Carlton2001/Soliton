use super::*;

fn wav(path: &Path) {
    crate::metadata::tests::wav(path);
}

#[test]
fn expands_folders_in_track_order_without_indexing() {
    let dir = tempfile::tempdir().unwrap();
    let album = dir.path().join("Artist/Album");
    for name in ["10 - Ten.wav", "2 - Two.wav", "1 - One.wav"] {
        wav(&album.join(name));
    }
    std::fs::write(album.join("cover.jpg"), b"x").unwrap();
    std::fs::write(album.join("broken.flac"), b"x").unwrap();
    let single = dir.path().join("Single/01 - Alone.wav");
    wav(&single);
    let covers = dir.path().join("cache");
    let loose = read_loose_files(
        &[album.clone(), single.clone(), dir.path().join("missing")],
        &covers,
        &Cancellation::default(),
        100,
    );
    let titles: Vec<_> = loose
        .tracks
        .iter()
        .map(|t| t.metadata.title.as_str())
        .collect();
    assert_eq!(titles, ["One", "Two", "Ten", "Alone"]);
    assert!(loose.tracks.iter().all(|t| t.id < 0 && t.available));
    assert_eq!(loose.errors.len(), 2);
    assert!(!loose.truncated);
    let limited = read_loose_files(&[album], &covers, &Cancellation::default(), 2);
    assert!(limited.truncated);
    assert_eq!(limited.tracks.len(), 2);
}
