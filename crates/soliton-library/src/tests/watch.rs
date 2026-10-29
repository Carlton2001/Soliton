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

#[test]
fn native_watcher_observes_files_before_and_after_source_reactivation() {
    let directory = tempfile::tempdir().unwrap();
    let mut source = Source {
        id: 1,
        path: directory.path().canonicalize().unwrap(),
        available: true,
    };
    let (sender, events) = crossbeam_channel::bounded(256);
    let mut watcher = SourceWatcher::new(sender).unwrap();
    assert!(
        watcher
            .sync(std::slice::from_ref(&source), |_| true)
            .is_empty()
    );
    let await_path = |path: &Path| {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match events
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("Native watcher did not report the new audio file")
            {
                WatchEvent::Paths(paths) if paths.iter().any(|changed| changed == path) => break,
                WatchEvent::Paths(_) => {}
                WatchEvent::Overflow => panic!("Unexpected overflow for one synthetic file"),
            }
        }
    };
    let first = source.path.join("first.wav");
    std::fs::write(&first, b"synthetic watcher fixture").unwrap();
    await_path(&first);

    source.available = false;
    assert!(
        watcher
            .sync(std::slice::from_ref(&source), |_| true)
            .is_empty()
    );
    source.available = true;
    assert!(
        watcher
            .sync(std::slice::from_ref(&source), |_| true)
            .is_empty()
    );
    let second = source.path.join("after-reactivation.wav");
    std::fs::write(&second, b"synthetic watcher fixture").unwrap();
    await_path(&second);
}

/// Reruns itself in a user namespace with a lowered inotify limit, so the
/// limit is reached without changing the system's.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "Needs unprivileged user namespaces; reruns itself under unshare"]
fn watch_limit_releases_partial_watches() {
    const CHILD: &str = "SOLITON_WATCH_LIMIT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let status = std::process::Command::new("unshare")
            .args(["-Ur", "sh", "-c"])
            .arg(r#"echo 8 > /proc/sys/user/max_inotify_watches && exec "$0" "$@""#)
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", "--ignored", "--nocapture"])
            .arg("watch::tests::watch_limit_releases_partial_watches")
            .env(CHILD, "1")
            .status()
            .expect("unshare from util-linux");
        assert!(status.success());
        return;
    }
    let source = |id, folders| {
        let directory = tempfile::tempdir().unwrap();
        for folder in 0..folders {
            std::fs::create_dir(directory.path().join(format!("{folder:02}"))).unwrap();
        }
        let source = Source {
            id,
            path: directory.path().canonicalize().unwrap(),
            available: true,
        };
        (directory, source)
    };
    let (_large_dir, large) = source(1, 20);
    let (_small_dir, small) = source(2, 3);
    let (sender, events) = crossbeam_channel::bounded(256);
    let mut watcher = SourceWatcher::new(sender).unwrap();
    let errors = watcher.sync(std::slice::from_ref(&large), |_| true);
    assert_eq!(errors.len(), 1, "{errors:?}");
    // The large source is left to scans: none of its folders is still watched.
    std::fs::write(large.path.join("00/large.wav"), b"x").unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(events.try_iter().next().is_none());
    // Its released quota covers the small source, while the large one is retried.
    let both = [large.clone(), small.clone()];
    assert_eq!(watcher.sync(&both, |_| true).len(), 1);
    let file = small.path.join("02/small.wav");
    std::fs::write(&file, b"x").unwrap();
    let event = events.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(event, WatchEvent::Paths(paths) if paths.contains(&file)));
}
