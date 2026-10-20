use super::*;
#[path = "support.rs"]
mod support;

#[test]
fn commands_without_a_device_preserve_stopped_state_and_validate_volume() {
    let mut worker = Worker {
        queue: Queue::default(),
        playback: Playback::default(),
        session: None,
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
fn missing_file_reports_failed_state_without_panicking() {
    let mut worker = Worker {
        queue: Queue::default(),
        playback: Playback::default(),
        session: None,
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
    let engine = Engine::start(0.).unwrap();
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
            tracks: vec![track.clone(), track],
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
    engine.send(Command::Resume).unwrap();
    wait(&|p| p.state == PlaybackState::Playing && p.position_ms > 2100);
    engine.send(Command::Next).unwrap();
    wait(&|p| p.state == PlaybackState::Playing && p.position_ms < 500);
    engine.send(Command::Stop).unwrap();
    wait(&|p| p.state == PlaybackState::Stopped && p.position_ms == 0);
}
