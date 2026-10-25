use super::*;

fn paths(names: &[&str]) -> WatchEvent {
    WatchEvent::Paths(names.iter().map(PathBuf::from).collect())
}

#[test]
fn coalesces_bursts_until_quiet_or_longest_delay() {
    let start = Instant::now();
    let second = Duration::from_secs(1);
    let mut coalescer = Coalescer::new(2 * second, 10 * second);
    assert!(coalescer.deadline().is_none());
    // Vanished paths are kept: they may have been audio files or folders.
    coalescer.push(paths(&["missing/a.flac", "missing/b"]), start);
    coalescer.push(paths(&["missing/a.flac"]), start + second);
    assert!(coalescer.take(start + 2 * second).is_none());
    let batch = coalescer.take(start + 3 * second).unwrap().unwrap();
    assert_eq!(batch.len(), 2);
    assert!(coalescer.take(start + 20 * second).is_none());
    // Continuous activity is flushed after the longest delay.
    for step in 0..12 {
        coalescer.push(paths(&["missing/c.flac"]), start + step * second);
    }
    assert!(coalescer.take(start + 9 * second).is_none());
    assert!(coalescer.take(start + 10 * second).is_some());
}

#[test]
fn ignores_existing_non_audio_files_and_reports_overflow() {
    let dir = tempfile::tempdir().unwrap();
    let cover = dir.path().join("cover.jpg");
    std::fs::write(&cover, b"x").unwrap();
    let start = Instant::now();
    let mut coalescer = Coalescer::new(Duration::ZERO, Duration::ZERO);
    coalescer.push(WatchEvent::Paths(vec![cover]), start);
    assert!(coalescer.take(start).is_none());
    coalescer.push(WatchEvent::Paths(vec![dir.path().into()]), start);
    assert_eq!(coalescer.take(start).unwrap().unwrap().len(), 1);
    coalescer.push(paths(&["missing/a.flac"]), start);
    coalescer.push(WatchEvent::Overflow, start);
    assert_eq!(coalescer.take(start), Some(None));
}

#[test]
fn maps_paths_to_the_owning_source_folders() {
    let dir = tempfile::tempdir().unwrap();
    let album = dir.path().join("music/Artist/Album");
    std::fs::create_dir_all(&album).unwrap();
    let sources = [
        Source {
            id: 1,
            path: dir.path().join("music"),
            available: true,
        },
        Source {
            id: 2,
            path: dir.path().join("other"),
            available: true,
        },
    ];
    let changed = [
        album.join("01.flac"),
        album.clone(),
        dir.path().join("elsewhere/01.flac"),
        dir.path().join("other/Gone/01.flac"),
    ];
    assert_eq!(
        scopes(&sources, &changed),
        vec![(1, vec![album]), (2, vec![dir.path().join("other/Gone")])]
    );
}
