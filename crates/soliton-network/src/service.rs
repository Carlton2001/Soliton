use crate::{Command, Snapshot, Status, controller::Controller, discovery};
use anyhow::{Context, Result};
use std::time::Duration;
use tokio::sync::{mpsc, watch};

const OPERATION_TIMEOUT: Duration = Duration::from_secs(12);

struct Request {
    command: Command,
    reply: crossbeam_channel::Sender<Result<Option<Status>>>,
}

/// Nonblocking discovery plus renderer RPCs. `call` is for the audio worker only;
/// it must never be invoked by GPUI or from a real-time audio callback.
pub struct Network {
    commands: mpsc::Sender<Request>,
    refresh: mpsc::Sender<()>,
    snapshots: watch::Receiver<Snapshot>,
    stop: watch::Sender<bool>,
    done: crossbeam_channel::Receiver<()>,
}

impl Network {
    pub fn start() -> Result<Self> {
        let (commands, requests) = mpsc::channel(8);
        let (refresh, scans) = mpsc::channel(1);
        let (updates, snapshots) = watch::channel(Snapshot {
            discovering: true,
            ..Default::default()
        });
        let (stop, stopped) = watch::channel(false);
        let (finished, done) = crossbeam_channel::bounded(1);
        std::thread::Builder::new()
            .name("soliton-network".into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .max_blocking_threads(8)
                    .enable_all()
                    .build();
                match runtime {
                    Ok(runtime) => {
                        runtime.block_on(run(requests, scans, updates, stopped));
                        runtime.shutdown_timeout(Duration::from_secs(1));
                    }
                    Err(error) => {
                        updates.send_modify(|s| {
                            s.discovering = false;
                            s.discovery_error = Some(error.to_string());
                        });
                    }
                }
                let _ = finished.try_send(());
            })?;
        Ok(Self {
            commands,
            refresh,
            snapshots,
            stop,
            done,
        })
    }

    pub fn refresh(&self) -> Result<()> {
        match self.refresh.try_send(()) {
            Ok(()) | Err(mpsc::error::TrySendError::Full(())) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn take_update(&mut self) -> Option<Snapshot> {
        self.snapshots
            .has_changed()
            .ok()
            .filter(|changed| *changed)
            .map(|_| self.snapshots.borrow_and_update().clone())
    }

    pub fn call(&self, command: Command) -> Result<Option<Status>> {
        let (reply, result) = crossbeam_channel::bounded(1);
        self.commands
            .try_send(Request { command, reply })
            .map_err(|_| anyhow::anyhow!("Network command queue unavailable"))?;
        result
            .recv_timeout(Duration::from_secs(16))
            .context("Network command timed out")?
    }

    pub fn shutdown(self) {
        let _ = self.stop.send(true);
        let _ = self.done.recv_timeout(Duration::from_secs(15));
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}

async fn run(
    mut requests: mpsc::Receiver<Request>,
    scans: mpsc::Receiver<()>,
    updates: watch::Sender<Snapshot>,
    mut stopped: watch::Receiver<bool>,
) {
    let (found, mut discovered) = watch::channel(discovery::Discovered::default());
    let discovery_task = tokio::spawn(discovery::run(scans, found));
    let mut controller = Controller::default();
    let mut discovery_alive = true;
    let mut interval = tokio::time::interval(Duration::from_millis(500));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            biased;
            _ = stopped.changed() => break,
            request = requests.recv() => {
                let Some(request) = request else { break; };
                let affects_media = matches!(request.command, Command::Load { .. } | Command::Play | Command::Stop | Command::Seek(_));
                let result = tokio::time::timeout(OPERATION_TIMEOUT, controller.command(request.command)).await
                    .map_err(|_| anyhow::anyhow!("Network operation timed out")).and_then(|r| r);
                if affects_media && let Err(error) = &result { controller.fail(anyhow::anyhow!("{error:#}")).await; }
                updates.send_modify(|s| s.status = controller.status.clone());
                let _ = request.reply.try_send(result);
            }
            result = discovered.changed(), if discovery_alive => {
                if result.is_err() {
                    discovery_alive = false;
                    updates.send_modify(|s| { s.discovering = false; s.discovery_error = Some("Network discovery stopped".into()); });
                    continue;
                }
                let found = discovered.borrow_and_update().clone();
                updates.send_modify(|s| { s.devices = std::sync::Arc::new(found.devices); s.discovery_error = found.error; s.discovering = found.scanning; });
            }
            _ = interval.tick() => {
                let result = tokio::time::timeout(OPERATION_TIMEOUT, controller.poll()).await
                    .map_err(|_| anyhow::anyhow!("Network receiver stopped responding")).and_then(|r| r);
                if let Err(error) = result { controller.fail(error).await; }
                updates.send_modify(|s| s.status = controller.status.clone());
            }
        }
    }
    controller.disconnect().await;
    // Dropping the refresh receiver ends discovery and shuts down its mDNS daemon.
    discovery_task.abort();
}
