use super::run;
use crate::service::{Event, Work};
use crossbeam_channel::{Receiver, bounded, unbounded};
use soliton_library::{Cancellation, Library};
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
    wait_for(&events, "offline source", |e| {
        matches!(e, Event::SourceUnavailable(_))
    });
    let offline = Instant::now();
    std::fs::rename(&parked, &music).unwrap();
    let available = || {
        Library::open(&root)
            .unwrap()
            .sources()
            .unwrap()
            .iter()
            .all(|source| source.available)
    };
    while !available() {
        assert!(
            offline.elapsed() < Duration::from_secs(15),
            "Source did not return"
        );
        let _ = events.recv_timeout(Duration::from_millis(200));
    }
    background.cancel();
    drop(work);
    worker.join().unwrap();
}
