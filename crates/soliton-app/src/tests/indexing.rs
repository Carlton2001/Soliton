use super::{LONGEST, QUIET, Silence, Worker, run};
use crate::service::{Event, Work};
use crossbeam_channel::{Receiver, bounded, unbounded};
use soliton_core::failure::{Failure, Kind};
use soliton_library::{Cancellation, Coalescer, Library};
use std::{
    path::Path,
    time::{Duration, Instant},
};

/// One second of 16-bit mono silence.
fn wav(path: &Path) {
    let samples = 8000u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + samples * 2).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&8000u32.to_le_bytes());
    bytes.extend_from_slice(&16000u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(samples * 2).to_le_bytes());
    bytes.resize(bytes.len() + samples as usize * 2, 0);
    std::fs::write(path, bytes).unwrap();
}

fn wait_for(events: &Receiver<Event>, what: &str, matches: impl Fn(&Event) -> bool) {
    let start = Instant::now();
    loop {
        let left = Duration::from_secs(20).saturating_sub(start.elapsed());
        assert!(!left.is_zero(), "{what} timed out");
        if let Ok(event) = events.recv_timeout(left)
            && matches(&event)
        {
            return;
        }
    }
}

#[test]
#[ignore = "Takes several seconds: waits for the offline and return probes"]
fn a_remounted_source_returns_without_a_manual_scan() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("library");
    let music = dir.path().join("music");
    let parked = dir.path().join("unmounted");
    std::fs::create_dir_all(music.join("Album")).unwrap();
    wav(&music.join("Album/01.wav"));
    let (work, received) = unbounded();
    let (sent, events) = bounded(256);
    let background = Cancellation::default();
    let worker = std::thread::spawn({
        let (root, background) = (root.clone(), background.clone());
        move || run(&root, received, sent, background)
    });
    work.send(Work::Scan(vec![music.clone()], Cancellation::default()))
        .unwrap();
    wait_for(&events, "initial scan", |e| matches!(e, Event::Finished(_)));
    // Renaming the folder away stands for unmounting its volume.
    std::fs::rename(&music, &parked).unwrap();
    let library = Library::open_reader(&root).unwrap();
    let id = library.sources().unwrap()[0].id;
    wait_for(
        &events,
        "offline source",
        |e| matches!(e, Event::SourceUnavailable(source_id, _) if *source_id == id),
    );
    std::fs::rename(&parked, &music).unwrap();
    wait_for(
        &events,
        "returning source",
        |e| matches!(e, Event::SourceAlertResolved(source_id) if *source_id == id),
    );
    assert!(library.sources().unwrap()[0].available);
    background.cancel();
    drop(work);
    worker.join().unwrap();
}

#[test]
fn a_probe_clears_the_timeout_of_a_source_that_was_still_available() {
    let dir = tempfile::tempdir().unwrap();
    let music = dir.path().join("music");
    std::fs::create_dir(&music).unwrap();
    let mut library = Library::open(&dir.path().join("data")).unwrap();
    let id = library.add_source(&music).unwrap();
    let (_work, requests) = unbounded();
    let (sent, events) = unbounded();
    let now = Instant::now();
    let mut worker = Worker {
        prober: library.prober(),
        library,
        work: requests,
        deferred: Default::default(),
        events: sent,
        watcher: None,
        changes: Coalescer::new(QUIET, LONGEST),
        silence: Silence::default(),
        timed_out: Default::default(),
        background: Cancellation::default(),
        next_probe: now,
        next_return_probe: now,
        next_periodic: now,
    };
    let failure = Failure::new(Kind::SourceNotResponding).about(music.display());
    worker.report_source_scan_failure(id, failure.clone());
    assert!(
        matches!(events.try_recv(), Ok(Event::SourceTimedOut(source, value))
        if source == id && value == failure)
    );
    assert!(worker.library.sources().unwrap()[0].available);

    worker.probe(false);

    assert!(matches!(events.try_recv(), Ok(Event::SourceAlertResolved(source)) if source == id));
    worker.probe(false);
    assert!(events.is_empty());
}

#[test]
fn a_silent_source_goes_offline_only_after_the_limit_and_a_refusal_at_once() {
    let limit = Duration::from_secs(30);
    let start = Instant::now();
    let mut silence = Silence::default();
    assert!(!silence.unreachable(1, None, start, limit));
    assert!(!silence.unreachable(1, None, start + limit / 2, limit));
    // An answer in between restarts the wait, as when sleeping disks spin up.
    assert!(!silence.unreachable(1, Some(true), start + limit / 2, limit));
    assert!(!silence.unreachable(1, None, start + limit, limit));
    assert!(silence.unreachable(1, None, start + limit * 2, limit));
    assert!(silence.unreachable(2, Some(false), start, limit));
    silence.forget(1);
    assert!(!silence.unreachable(1, None, start + limit * 3, limit));
}
