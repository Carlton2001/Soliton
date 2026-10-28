use super::*;
use crate::test_support as support;

#[test]
fn restored_playlist_deduplicates_without_losing_position_or_provenance() {
    let track = soliton_core::model::Track {
        id: 7,
        source_id: 1,
        path: "unopened.flac".into(),
        metadata: soliton_core::model::Metadata {
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
    let track = soliton_core::model::Track {
        id: 1,
        source_id: 1,
        path: "unopened.flac".into(),
        metadata: soliton_core::model::Metadata {
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
    let track = soliton_core::model::Track {
        id: 1,
        source_id: 1,
        path: "unopened.flac".into(),
        metadata: soliton_core::model::Metadata {
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
                device_label: "Disconnected DAC".into(),
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
    let track = soliton_core::model::Track {
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
    let track = soliton_core::model::Track {
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
    let next = soliton_core::model::Track {
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
    // The device actually linked, once the sound server reports it.
    let named = wait(&|p| {
        !p.outputs.is_empty() && p.output.as_ref().is_some_and(|o| !o.device_name.is_empty())
    });
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
            "Soliton nonexistent device fixture".into(),
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
fn unavailable_preference_pauses_then_resumes_on_the_default_and_output_loss_pauses() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.wav");
    support::wav(&path, 44100, 1);
    let preferred = "Soliton nonexistent device fixture";
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
            tracks: vec![soliton_core::model::Track {
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
    // Nothing plays until the user resumes: never straight to another output.
    assert!(worker.interrupt(&error));
    assert_eq!(worker.playback.state, PlaybackState::Paused);
    assert_eq!(
        worker.playback.interruption,
        Some(Failure::with(Kind::OutputUnavailable, preferred))
    );
    assert!(worker.playback.fallback);
    assert!(worker.playback.output.is_none());
    assert!(worker.playback.error.is_none());
    worker.command(Command::Resume).unwrap();
    assert_eq!(worker.playback.state, PlaybackState::Playing);
    assert!(worker.playback.output.is_some());
    assert!(worker.playback.interruption.is_none());
    // The preference is kept for its return.
    assert_eq!(worker.playback.selected_output.as_deref(), Some(preferred));
    for _ in 0..10 {
        worker.pump().unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }
    let position = worker.playback.position_ms;
    // Simulate the callback's disconnect signal without changing system devices.
    worker
        .session
        .as_ref()
        .unwrap()
        .output
        .failed
        .store(true, Ordering::Relaxed);
    let error = worker.pump().unwrap_err();
    worker.recover(error);
    assert_eq!(worker.playback.state, PlaybackState::Paused);
    assert_eq!(
        worker.playback.interruption.as_ref().map(|f| f.kind),
        Some(Kind::OutputDisconnected)
    );
    assert!(worker.playback.output.is_none());
    assert_eq!(worker.playback.position_ms, position);
    assert_eq!(worker.playback.track.as_ref().unwrap().id, 1);
    worker.command(Command::Resume).unwrap();
    assert!(worker.playback.interruption.is_none());
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
fn a_missing_output_pauses_at_the_position_and_its_return_ends_the_fallback() {
    let track = soliton_core::model::Track {
        id: 1,
        source_id: 1,
        path: "unopened.flac".into(),
        metadata: soliton_core::model::Metadata {
            duration_ms: 60_000,
            ..Default::default()
        },
        cover: None,
        available: true,
    };
    let mut worker = Worker {
        playback: Playback {
            selected_output: Some("headset".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    worker
        .command(Command::Load {
            tracks: vec![track],
            start: 0,
            position_ms: 12_000,
        })
        .unwrap();
    let lost = Failure::with(Kind::OutputDisconnected, "Headset");
    assert!(worker.interrupt(&lost.clone().into()));
    assert_eq!(worker.playback.state, PlaybackState::Paused);
    assert_eq!(worker.resume_at, Some(12_000));
    assert_eq!(worker.output_target(), None);
    // Track and network errors are not output interruptions.
    assert!(!worker.interrupt(&Failure::new(Kind::UnreadableAudio).into()));
    let speakers = crate::OutputDevice {
        name: "speakers".into(),
        label: "Speakers".into(),
        bluetooth: false,
        selectable: true,
    };
    let headset = crate::OutputDevice {
        name: "headset".into(),
        label: "Headset".into(),
        bluetooth: true,
        selectable: true,
    };
    // A listing sent before the loss was noticed still contains the output.
    let stale = vec![speakers.clone(), headset.clone()];
    worker.playback.outputs = Arc::new(stale.clone());
    worker.outputs_changed(&stale);
    assert!(worker.playback.fallback);
    worker.playback.outputs = Arc::new(vec![speakers.clone()]);
    worker.outputs_changed(&stale);
    assert!(worker.playback.fallback);
    worker.playback.outputs = Arc::new(vec![speakers.clone(), headset]);
    worker.outputs_changed(&[speakers]);
    assert!(!worker.playback.fallback);
    assert_eq!(worker.output_target(), Some("headset"));
    // Shown until playback resumes, even once the output is back.
    assert_eq!(worker.playback.interruption, Some(lost));
    worker.command(Command::Stop).unwrap();
    assert!(worker.playback.interruption.is_none());
}

#[test]
fn loading_a_queue_stays_paused_at_its_position_without_a_device() {
    let track = soliton_core::model::Track {
        id: 1,
        source_id: 1,
        path: "missing.flac".into(),
        metadata: soliton_core::model::Metadata {
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

fn missing(id: i64, name: &str) -> soliton_core::model::Track {
    soliton_core::model::Track {
        id,
        source_id: 1,
        path: name.into(),
        metadata: Default::default(),
        cover: None,
        available: true,
    }
}

#[test]
fn automatic_playback_skips_unreadable_tracks_until_the_end_of_the_queue() {
    let mut worker = Worker::default();
    worker
        .queue
        .set(vec![missing(1, "a.wav"), missing(2, "b.wav")], 0);
    let error = worker.start_current().unwrap_err();
    worker.recover(error);
    // Both tracks were tried; the queue then ends instead of failing.
    assert_eq!(worker.playback.state, PlaybackState::Stopped);
    assert!(worker.playback.error.is_none());
    let warning = worker.playback.warning.unwrap();
    assert_eq!(warning.kind, Kind::UnreadableAudio);
    assert!(warning.detail.starts_with("b.wav"));
    assert_eq!(worker.queue.current().unwrap().id, 2);
}

#[test]
fn skipping_stops_after_one_round_and_never_for_output_failures() {
    let mut worker = Worker::default();
    worker
        .queue
        .set(vec![missing(1, "a.wav"), missing(2, "b.wav")], 0);
    worker.queue.repeat = crate::Repeat::All;
    let error = worker.start_current().unwrap_err();
    worker.recover(error);
    assert_eq!(worker.playback.state, PlaybackState::Failed);
    assert_eq!(worker.playback.error.unwrap().kind, Kind::UnreadableAudio);

    let mut worker = Worker::default();
    worker
        .queue
        .set(vec![missing(1, "a.wav"), missing(2, "b.wav")], 0);
    worker.recover(Failure::new(Kind::OutputDisconnected).into());
    assert_eq!(worker.playback.state, PlaybackState::Failed);
    assert!(worker.playback.warning.is_none());
    assert_eq!(worker.queue.current().unwrap().id, 1);
}
