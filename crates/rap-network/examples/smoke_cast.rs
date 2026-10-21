//! Explicit, muted native QA. Uses an existing synthetic fixture, restores the
//! receiver volume and closes RAP's media-receiver app on success or failure.
use anyhow::{Context, Result, ensure};
use rap_network::{Command, Network, State, Status};
use std::time::{Duration, Instant};

fn wait(network: &mut Network, predicate: impl Fn(&Status) -> bool) -> Result<Status> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(8) {
        if let Some(status) = network.take_update().and_then(|s| s.status) {
            ensure!(status.state != State::Failed, "{:?}", status.error);
            if predicate(&status) {
                return Ok(status);
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    anyhow::bail!("Receiver playback status timed out")
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(false)
        .init();
    let args: Vec<_> = std::env::args().collect();
    ensure!(
        args.len() == 3,
        "Usage: smoke_cast <discovered-id> <synthetic-audio-file>"
    );
    let mut network = Network::start()?;
    let started = Instant::now();
    let device = loop {
        if let Some(snapshot) = network.take_update()
            && let Some(device) = snapshot.devices.iter().find(|d| d.id == args[1])
        {
            break device.clone();
        }
        ensure!(
            started.elapsed() < Duration::from_secs(15),
            "Receiver not discovered"
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    let runtime = tokio::runtime::Runtime::new()?;
    let original_volume = runtime.block_on(async {
        let client =
            oxicast::CastClient::builder(device.address.ip().to_string(), device.address.port())
                .auto_reconnect(false)
                .request_timeout(Duration::from_secs(3))
                .connect()
                .await?;
        let status = client.receiver_status().await?;
        // Do not interrupt an existing media application during automated QA.
        ensure!(
            status
                .applications
                .iter()
                .all(|app| app.display_name == "Backdrop"),
            "Receiver is in use; no media was loaded"
        );
        Ok::<_, anyhow::Error>(status.volume.level)
    })?;
    let result = (|| {
        network
            .call(Command::Connect(device))
            .context("Connect Cast receiver")?;
        let track = rap_core::model::Track {
            id: 1,
            source_id: 0,
            path: std::fs::canonicalize(&args[2])?,
            cover: None,
            available: true,
            metadata: rap_core::model::Metadata {
                title: "RAP — validation Cast".into(),
                artist: "RAP tests".into(),
                album: "Synthetic fixture".into(),
                ..Default::default()
            },
        };
        let loaded = network
            .call(Command::Load {
                track,
                position_ms: 0,
                autoplay: false,
                volume: 0.,
            })
            .context("Load Cast media at zero volume")?
            .context("No load response")?;
        ensure!(
            matches!(loaded.state, State::Paused | State::Buffering),
            "Unexpected load state: {:?}",
            loaded.state
        );
        network.call(Command::Play)?;
        wait(&mut network, |s| {
            s.state == State::Playing && s.position_ms > 200
        })?;
        let paused = network.call(Command::Pause)?.context("No pause response")?;
        ensure!(paused.state == State::Paused, "Pause was not acknowledged");
        let seek = network
            .call(Command::Seek(3000))?
            .context("No seek response")?;
        ensure!(
            seek.position_ms.abs_diff(3000) < 1000,
            "Seek did not move to 3 seconds"
        );
        network.call(Command::Play)?;
        wait(&mut network, |s| {
            s.state == State::Playing && s.position_ms > 3200
        })?;
        network.call(Command::Stop)?;
        println!("Cast native load, play, pause, seek, resume and stop: OK");
        anyhow::Ok(())
    })();
    let _ = network.call(Command::Stop);
    let _ = network.call(Command::Volume(original_volume));
    let _ = network.call(Command::Disconnect);
    network.shutdown();
    result
}
