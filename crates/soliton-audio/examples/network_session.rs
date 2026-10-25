//! Explicit native QA on an idle Cast receiver, using silent synthetic audio.
//! Exercises the common queue, repeat and switching back to a local output.
use anyhow::{Context, Result, ensure};
use soliton_audio::{Command, Engine, Playback, PlaybackState, Repeat};
use std::time::{Duration, Instant};

fn wait(engine: &Engine, predicate: impl Fn(&Playback) -> bool) -> Result<Playback> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        let state = match engine.events.recv_timeout(Duration::from_secs(2)) {
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
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 3,
        "Usage: network_session <Cast-id> <12-second-silent-audio>"
    );
    let engine = Engine::start(0., None)?;
    engine.send(Command::DiscoverNetworkOutputs)?;
    let devices = wait(&engine, |p| {
        p.network_outputs.iter().any(|d| d.id == args[1])
    })?;
    let device = devices
        .network_outputs
        .iter()
        .find(|d| d.id == args[1])
        .unwrap();
    let runtime = tokio::runtime::Runtime::new()?;
    let original_volume = runtime.block_on(async {
        let client =
            oxicast::CastClient::builder(device.address.ip().to_string(), device.address.port())
                .auto_reconnect(false)
                .request_timeout(Duration::from_secs(4))
                .connect()
                .await?;
        let state = client.receiver_status().await?;
        ensure!(
            state
                .applications
                .iter()
                .all(|a| a.display_name == "Backdrop"),
            "Receiver is in use; QA did not launch media"
        );
        anyhow::Ok(state.volume.level)
    })?;
    let result = (|| {
        engine.send(Command::SelectNetworkOutput(args[1].clone()))?;
        wait(&engine, |p| {
            p.network_output.is_some() && !p.output_changing
        })?;
        let track = soliton_core::model::Track {
            id: 1,
            source_id: 0,
            path: std::fs::canonicalize(&args[2])?,
            cover: None,
            available: true,
            metadata: soliton_core::model::Metadata {
                title: "Soliton queue validation".into(),
                duration_ms: 12_000,
                ..Default::default()
            },
        };
        let mut second = track.clone();
        second.id = 2;
        engine.send(Command::Play {
            tracks: vec![track, second],
            start: 0,
        })?;
        wait(&engine, |p| {
            p.state == PlaybackState::Playing && p.position_ms > 200
        })?;
        eprintln!("Pause first track");
        engine.send(Command::Pause)?;
        wait(&engine, |p| p.state == PlaybackState::Paused)?;
        eprintln!("Seek paused first track");
        engine.send(Command::Seek(3000))?;
        wait(&engine, |p| {
            p.state == PlaybackState::Paused && p.position_ms.abs_diff(3000) < 500
        })?;
        eprintln!("Resume first track");
        engine.send(Command::Resume)?;
        wait(&engine, |p| {
            p.state == PlaybackState::Playing && p.position_ms > 3200
        })?;
        eprintln!("Next track");
        engine.send(Command::Next)?;
        wait(&engine, |p| {
            p.track.as_ref().is_some_and(|t| t.id == 2) && p.state == PlaybackState::Playing
        })?;
        eprintln!("Previous track");
        engine.send(Command::Previous)?;
        wait(&engine, |p| {
            p.track.as_ref().is_some_and(|t| t.id == 1) && p.state == PlaybackState::Playing
        })?;
        eprintln!("Repeat track from near end");
        engine.send(Command::Repeat(Repeat::Track))?;
        engine.send(Command::Seek(11_000))?;
        wait(&engine, |p| p.position_ms >= 10_500)?;
        wait(&engine, |p| {
            p.position_ms < 1000
                && p.track.as_ref().is_some_and(|t| t.id == 1)
                && p.state == PlaybackState::Playing
        })?;
        eprintln!("Repeat all, manual next and wrap");
        engine.send(Command::Repeat(Repeat::All))?;
        engine.send(Command::Next)?;
        wait(&engine, |p| p.track.as_ref().is_some_and(|t| t.id == 2))?;
        engine.send(Command::Next)?;
        wait(&engine, |p| p.track.as_ref().is_some_and(|t| t.id == 1))?;
        engine.send(Command::Pause)?;
        wait(&engine, |p| p.state == PlaybackState::Paused)?;
        eprintln!("Switch paused track to local output");
        engine.send(Command::SelectOutput(None))?;
        wait(&engine, |p| {
            p.network_output.is_none() && p.output.is_some() && p.state == PlaybackState::Paused
        })?;
        engine.send(Command::Resume)?;
        wait(&engine, |p| {
            p.state == PlaybackState::Playing && p.position_ms > 100
        })?;
        engine.send(Command::Stop)?;
        wait(&engine, |p| {
            p.state == PlaybackState::Stopped && p.output.is_none()
        })?;
        println!(
            "Native network queue, repeat, next/previous and return to paused local output: OK"
        );
        anyhow::Ok(())
    })();
    let address = device.address;
    engine.shutdown();
    // Restore the receiver's pre-test volume after all playback has stopped.
    let restored = runtime.block_on(async {
        let client = oxicast::CastClient::builder(address.ip().to_string(), address.port())
            .auto_reconnect(false)
            .request_timeout(Duration::from_secs(4))
            .connect()
            .await?;
        client.set_volume(original_volume).await?;
        anyhow::Ok(())
    });
    restored.context("Restore receiver volume after native QA")?;
    result
}
