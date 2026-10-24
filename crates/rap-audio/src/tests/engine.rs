use super::*;
#[path = "support.rs"]
mod support;

#[test]
fn restored_playlist_deduplicates_without_losing_position_or_provenance() {
    let track = rap_core::model::Track {
        id: 7,
        source_id: 1,
        path: "unopened.flac".into(),
        metadata: rap_core::model::Metadata {
            duration_ms: 60_000,
            ..Default::default()
        },
        cover: None,
        available: true,
    };
    let mut worker = Worker::default();
    worker
        .command(Command::LoadSelection {
            entries: vec![(Some(20), track.clone()), (Some(21), track.clone())],
            start: 1,
            position_ms: 12_000,
        })
        .unwrap();
    assert_eq!(worker.snapshot().queue.entries.len(), 1);
    assert_eq!(worker.snapshot().queue.current_playlist_entry(), Some(20));
    assert_eq!(worker.playback.position_ms, 12_000);
    assert_eq!(worker.playback.state, PlaybackState::Paused);
    assert!(worker.session.is_none());
    worker
        .command(Command::Load {
            tracks: vec![track],
            start: 0,
            position_ms: 0,
        })
        .unwrap();
    assert_eq!(worker.snapshot().queue.current_playlist_entry(), None);
}

#[test]
fn editing_a_loaded_queue_does_not_open_outputs_or_reset_position() {
    let track = rap_core::model::Track {
        id: 1,
        source_id: 1,
        path: "unopened.flac".into(),
        metadata: rap_core::model::Metadata {
            duration_ms: 60_000,
            ..Default::default()
        },
        cover: None,
        available: true,
    };
    let mut worker = Worker::default();
    worker
        .command(Command::Enqueue {
            tracks: vec![track.clone()],
            next: false,
        })
        .unwrap();
    assert_eq!(worker.playback.state, PlaybackState::Paused);
    assert!(worker.session.is_none());
    worker.command(Command::Seek(12_000)).unwrap();
    let current = worker.snapshot().queue.current;
    let mut second = track.clone();
    second.id = 2;
    second.path = "second.flac".into();
    let mut third = track.clone();
    third.id = 3;
    third.path = "third.flac".into();
    worker
        .command(Command::Enqueue {
            tracks: vec![track, second, third],
            next: true,
        })
        .unwrap();
    let last = worker.snapshot().queue.entries[2].id;
    worker
        .command(Command::MoveQueued {
            id: last,
            before: Some(worker.snapshot().queue.entries[1].id),
        })
        .unwrap();
    worker.command(Command::RemoveQueued(last)).unwrap();
    worker.command(Command::ClearUpcoming).unwrap();
    assert_eq!(worker.snapshot().queue.current, current);
    assert_eq!(worker.snapshot().queue.entries.len(), 1);
    assert_eq!(worker.playback.position_ms, 12_000);
    assert_eq!(worker.playback.state, PlaybackState::Paused);
    assert!(worker.session.is_none());
}

#[test]
fn clearing_the_entire_queue_resets_current_track_and_position() {
    let track = rap_core::model::Track {
        id: 1,
        source_id: 1,
        path: "unopened.flac".into(),
        metadata: rap_core::model::Metadata {
            duration_ms: 60_000,
            ..Default::default()
        },
        cover: None,
        available: true,
    };
    let mut worker = Worker::default();
    worker
        .command(Command::Load {
            tracks: vec![track],
            start: 0,
            position_ms: 12_000,
        })
        .unwrap();
    assert_eq!(worker.playback.state, PlaybackState::Paused);
    worker
        .command(Command::Load {
            tracks: Vec::new(),
            start: 0,
            position_ms: 0,
        })
        .unwrap();
    let snapshot = worker.snapshot();
    assert!(snapshot.queue.entries.is_empty());
    assert!(snapshot.queue.current.is_none());
    assert!(snapshot.track.is_none());
    assert_eq!(snapshot.state, PlaybackState::Stopped);
    assert_eq!(snapshot.position_ms, 0);
    assert_eq!(snapshot.duration_ms, 0);
    assert!(worker.session.is_none());
}
#[test]
fn commands_without_a_device_preserve_stopped_state_and_validate_volume() {
    let mut worker = Worker {
        queue: Queue::default(),
        playback: Playback::default(),
        session: None,
        ..Default::default()
    };
    worker.command(Command::Pause).unwrap();
    assert_eq!(worker.playback.state, PlaybackState::Stopped);
    worker.command(Command::Seek(5000)).unwrap();
    assert_eq!(worker.playback.position_ms, 0);
    worker.command(Command::Volume(4.)).unwrap();
    assert_eq!(worker.playback.volume, 1.);
    worker.command(Command::Volume(f32::NAN)).unwrap();
    assert_eq!(worker.playback.volume, 1.);
    worker.command(Command::Stop).unwrap();
    assert_eq!(worker.playback.state, PlaybackState::Stopped);
}

#[test]
fn advancing_past_the_end_clears_output_details_but_keeps_preference() {
    let mut worker = Worker {
        queue: Queue::default(),
        playback: Playback {
            selected_output: Some("Disconnected DAC".into()),
            output: Some(crate::OutputInfo {
                device_name: "Disconnected DAC".into(),
                sample_rate: 48000,
                channels: 2,
                sample_format: "F32".into(),
            }),
            ..Default::default()
        },
        session: None,
        ..Default::default()
    };
    worker.command(Command::Next).unwrap();
    assert!(worker.playback.output.is_none());
    assert_eq!(
        worker.playback.selected_output.as_deref(),
        Some("Disconnected DAC")
    );
    worker.command(Command::SelectOutput(None)).unwrap();
    assert!(worker.playback.selected_output.is_none());
    assert_eq!(worker.playback.state, PlaybackState::Stopped);
}

#[test]
fn missing_file_reports_failed_state_without_panicking() {
    let mut worker = Worker {
        queue: Queue::default(),
        playback: Playback::default(),
        session: None,
        ..Default::default()
    };
    let track = rap_core::model::Track {
        id: 1,
        source_id: 1,
        path: "missing-track.wav".into(),
        metadata: Default::default(),
        cover: None,
        available: true,
    };
    let error = worker
        .command(Command::Play {
            tracks: vec![track],
            start: 0,
        })
        .unwrap_err();
    worker.fail(error);
    assert_eq!(worker.playback.state, PlaybackState::Failed);
    assert!(worker.playback.error.is_some());
}

#[test]
#[ignore = "Requires a native output device; run explicitly with volume zero"]
fn native_output_play_pause_seek_next_and_stop() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    support::wav(&path, 44100, 5);
    let track = rap_core::model::Track {
        id: 1,
        source_id: 1,
        path,
        metadata: Default::default(),
        cover: None,
        available: true,
    };
    // The queue keeps one occurrence per file: Next needs another file.
    let next_path = dir.path().join("next.wav");
    support::wav(&next_path, 44100, 5);
    let next = rap_core::model::Track {
        id: 2,
        path: next_path,
        ..track.clone()
    };
    let engine = Engine::start(0., None).unwrap();
    let wait = |predicate: &dyn Fn(&Playback) -> bool| {
        let start = Instant::now();
        loop {
            assert!(
                start.elapsed() < Duration::from_secs(8),
                "Audio event timed out"
            );
            let value = engine.events.recv_timeout(Duration::from_secs(2)).unwrap();
            assert_ne!(value.state, PlaybackState::Failed, "{:?}", value.error);
            if predicate(&value) {
                return value;
            }
        }
    };
    engine
        .send(Command::Play {
            tracks: vec![track, next],
            start: 0,
        })
        .unwrap();
    wait(&|p| p.state == PlaybackState::Playing && p.position_ms > 100);
    engine.send(Command::Pause).unwrap();
    let paused = wait(&|p| p.state == PlaybackState::Paused);
    let still = wait(&|p| p.state == PlaybackState::Paused);
    assert_eq!(paused.position_ms, still.position_ms);
    engine.send(Command::Seek(2000)).unwrap();
    wait(&|p| p.state == PlaybackState::Paused && p.position_ms == 2000);
    let named = wait(&|p| !p.outputs.is_empty());
    let output = named.output.unwrap();
    assert!(output.sample_rate > 0);
    assert!(output.channels > 0);
    assert!(!output.sample_format.is_empty());
    // Explicit selection while paused must keep the track and position.
    engine
        .send(Command::SelectOutput(Some(output.device_name.clone())))
        .unwrap();
    let selected = wait(&|p| p.selected_output.as_ref() == Some(&output.device_name));
    assert_eq!(selected.state, PlaybackState::Paused);
    assert_eq!(selected.position_ms, 2000);
    assert_eq!(selected.track.as_ref().unwrap().id, 1);
    // A rejected device must not stop the working stream or change preference.
    engine
        .send(Command::SelectOutput(Some(
            "RAP nonexistent device fixture".into(),
        )))
        .unwrap();
    let rejected = wait(&|p| p.output_error.is_some());
    assert_eq!(rejected.state, PlaybackState::Paused);
    assert_eq!(rejected.position_ms, 2000);
    assert_eq!(
        rejected.output.as_ref().unwrap().device_name,
        output.device_name
    );
    assert_eq!(rejected.selected_output, Some(output.device_name));
    engine.send(Command::Resume).unwrap();
    wait(&|p| p.state == PlaybackState::Playing && p.position_ms > 2100);
    engine.send(Command::SelectOutput(None)).unwrap();
    let restored = wait(&|p| p.selected_output.is_none() && p.output_error.is_none());
    assert_eq!(restored.state, PlaybackState::Playing);
    assert!(restored.position_ms >= 2000);
    assert!(restored.output.is_some());
    engine.send(Command::Next).unwrap();
    wait(&|p| p.state == PlaybackState::Playing && p.position_ms < 500);
    engine.send(Command::Stop).unwrap();
    let stopped = wait(&|p| p.state == PlaybackState::Stopped && p.position_ms == 0);
    assert!(stopped.output.is_none());
}

#[test]
#[ignore = "Requires a native output device; run explicitly with volume zero"]
fn unavailable_preference_never_falls_back_and_output_failure_can_be_recovered() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    support::wav(&path, 44100, 1);
    let preferred = "RAP nonexistent device fixture";
    let mut worker = Worker {
        queue: Queue::default(),
        playback: Playback {
            selected_output: Some(preferred.into()),
            volume: 0.,
            ..Default::default()
        },
        session: None,
        ..Default::default()
    };
    let error = worker
        .command(Command::Play {
            tracks: vec![rap_core::model::Track {
                id: 1,
                source_id: 1,
                path,
                metadata: Default::default(),
                cover: None,
                available: true,
            }],
            start: 0,
        })
        .unwrap_err();
    worker.fail(error);
    assert_eq!(
        worker.playback.error,
        Some(Failure::with(Kind::OutputUnavailable, preferred))
    );
    assert!(worker.playback.output.is_none());
    assert_eq!(worker.playback.selected_output.as_deref(), Some(preferred));
    worker.command(Command::SelectOutput(None)).unwrap();
    assert_eq!(worker.playback.state, PlaybackState::Failed);
    worker.command(Command::Resume).unwrap();
    assert_eq!(worker.playback.state, PlaybackState::Playing);
    assert!(worker.playback.output.is_some());
    // Simulate the callback's disconnect signal without changing system devices.
    worker
        .session
        .as_ref()
        .unwrap()
        .output
        .failed
        .store(true, Ordering::Relaxed);
    let error = worker.pump().unwrap_err();
    worker.fail(error);
    assert_eq!(worker.playback.state, PlaybackState::Failed);
    assert!(worker.playback.output.is_none());
    assert_eq!(worker.playback.track.as_ref().unwrap().id, 1);
    worker.command(Command::Resume).unwrap();
    assert!(worker.playback.error.is_none());
    assert_eq!(worker.playback.state, PlaybackState::Playing);
    worker.command(Command::Pause).unwrap();
    worker.command(Command::Seek(1000)).unwrap();
    worker.command(Command::Resume).unwrap();
    for _ in 0..20 {
        worker.pump().unwrap();
        if worker.playback.state == PlaybackState::Stopped {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(worker.playback.state, PlaybackState::Stopped);
    assert!(worker.playback.output.is_none());
}

#[test]
fn loading_a_queue_stays_paused_at_its_position_without_a_device() {
    let track = rap_core::model::Track {
        id: 1,
        source_id: 1,
        path: "missing.flac".into(),
        metadata: rap_core::model::Metadata {
            duration_ms: 60_000,
            ..Default::default()
        },
        cover: None,
        available: true,
    };
    let engine = Engine::start(0., None).unwrap();
    engine
        .send(Command::Load {
            tracks: vec![track],
            start: 0,
            position_ms: 90_000,
        })
        .unwrap();
    engine.send(Command::Seek(12_000)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let value = engine.events.recv_timeout(Duration::from_secs(2)).unwrap();
        if value.position_ms == 12_000 {
            assert_eq!(value.state, PlaybackState::Paused);
            assert_eq!(value.track.map(|t| t.id), Some(1));
            assert!(value.output.is_none());
            break;
        }
        assert!(Instant::now() < deadline, "Load was not applied");
    }
    engine.shutdown();
}
