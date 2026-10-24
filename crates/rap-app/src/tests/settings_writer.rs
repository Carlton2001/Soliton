use super::*;
use crossbeam_channel::bounded;

#[test]
fn newest_value_is_written_and_flush_waits_for_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let (events, errors) = bounded(4);
    let writer = SettingsWriter::start(path.clone(), events);
    for step in 0..100 {
        writer.save(Settings {
            volume: step as f32 / 100.,
            ..Default::default()
        });
    }
    writer
        .flush(
            Settings {
                volume: 0.25,
                ..Default::default()
            },
            Duration::from_secs(5),
        )
        .unwrap();
    assert_eq!(Settings::load(&path).unwrap().volume, 0.25);
    assert!(errors.try_recv().is_err());
}

#[test]
fn pending_value_is_written_when_the_last_writer_is_dropped() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings.json");
    let (events, _errors) = bounded(4);
    let writer = SettingsWriter::start(path.clone(), events);
    let shared = Arc::downgrade(&writer.0.0);
    writer.clone().save(Settings {
        volume: 0.5,
        ..Default::default()
    });
    drop(writer);
    // The worker owns the last strong reference until it exits.
    let started = std::time::Instant::now();
    while shared.strong_count() > 0 {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "Worker did not exit"
        );
        std::thread::yield_now();
    }
    assert_eq!(Settings::load(&path).unwrap().volume, 0.5);
}

#[test]
fn failures_reach_the_flush_or_the_event_queue() {
    let directory = tempfile::tempdir().unwrap();
    // A file where the parent directory should be: every write fails.
    let blocked = directory.path().join("blocked");
    std::fs::write(&blocked, b"").unwrap();
    let (events, errors) = bounded(4);
    let writer = SettingsWriter::start(blocked.join("settings.json"), events);
    assert!(
        writer
            .flush(Settings::default(), Duration::from_secs(5))
            .is_err()
    );
    assert!(errors.try_recv().is_err());
    writer.save(Settings::default());
    let reported = errors.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(reported, Event::Error(_)));
}
