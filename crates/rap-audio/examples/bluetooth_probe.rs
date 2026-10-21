//! Opt-in hardware QA: five seconds of quiet generated audio on an exact local
//! output name. Does not pair devices, change system volume or touch the library.
use anyhow::{Result, ensure};
use rap_audio::{Command, Engine, Playback, PlaybackState};
use std::time::{Duration, Instant};

#[path = "../src/tests/support.rs"]
mod support;

fn wait(engine: &Engine, predicate: impl Fn(&Playback) -> bool) -> Result<Playback> {
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline {
        let state = match engine.events.recv_timeout(Duration::from_secs(1)) {
            Ok(state) => state,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(error) => return Err(error.into()),
        };
        ensure!(state.state != PlaybackState::Failed, "{:?}", state.error);
        ensure!(state.output_error.is_none(), "{:?}", state.output_error);
        if predicate(&state) {
            return Ok(state);
        }
    }
    anyhow::bail!("Audio engine status timed out")
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 2,
        "Usage: bluetooth_probe <exact Windows audio output name>"
    );
    let name = &args[1];
    let engine = Engine::start(0.15, Some(name.clone()))?;
    let result = (|| {
        let outputs = wait(&engine, |state| !state.outputs.is_empty())?;
        ensure!(
            outputs
                .outputs
                .iter()
                .any(|d| &d.name == name && d.selectable),
            "Requested output is unavailable or ambiguous. Available outputs: {:?}",
            outputs.outputs
        );
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("rap-bluetooth-tone.wav");
        support::wav(&path, 44100, 5);
        engine.send(Command::Play {
            tracks: vec![rap_core::model::Track {
                id: 1,
                source_id: 0,
                path,
                metadata: Default::default(),
                cover: None,
                available: true,
            }],
            start: 0,
        })?;
        let playing = wait(&engine, |state| {
            state.state == PlaybackState::Playing && state.position_ms > 100
        })?;
        let info = playing.output.as_ref().expect("Playing output");
        ensure!(&info.device_name == name, "Playback used the wrong output");
        println!(
            "Selected output: {} / {} Hz / {} channels / {}",
            info.device_name, info.sample_rate, info.channels, info.sample_format
        );
        engine.send(Command::Pause)?;
        let paused = wait(&engine, |state| state.state == PlaybackState::Paused)?;
        let still = wait(&engine, |state| state.state == PlaybackState::Paused)?;
        ensure!(
            still.position_ms == paused.position_ms,
            "Pause advanced playback"
        );
        engine.send(Command::Resume)?;
        wait(&engine, |state| {
            state.state == PlaybackState::Playing && state.position_ms > paused.position_ms
        })?;
        let finished = wait(&engine, |state| state.state == PlaybackState::Stopped)?;
        ensure!(
            finished.selected_output.as_deref() == Some(name.as_str()),
            "Output preference was lost"
        );
        println!(
            "Exact output, playback progress, pause/resume and end of track OK. Audible sound and Bluetooth codec require separate confirmation."
        );
        Ok(())
    })();
    let _ = engine.send(Command::Stop);
    engine.shutdown();
    result
}
