//! Settings persistence on its own worker, so saving never waits behind library
//! scans. Only the newest unsaved value is kept: older ones are already obsolete.
use crate::service::Event;
use crossbeam_channel::{Sender, bounded};
use rap_core::{
    config::Settings,
    failure::{Failure, Kind},
};
use std::{
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, MutexGuard},
    time::Duration,
};

#[derive(Default)]
struct Pending {
    settings: Option<Settings>,
    flushes: Vec<Sender<anyhow::Result<()>>>,
    closed: bool,
}

#[derive(Default)]
struct Shared {
    pending: Mutex<Pending>,
    ready: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, Pending> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Closes the worker once the last writer is dropped, after pending values.
struct Handle(Arc<Shared>);

impl Drop for Handle {
    fn drop(&mut self) {
        self.0.lock().closed = true;
        self.0.ready.notify_one();
    }
}

#[derive(Clone)]
pub struct SettingsWriter(Arc<Handle>);

impl SettingsWriter {
    /// Write failures are reported as [`Event::Error`], unless a flush awaits them.
    pub fn start(path: PathBuf, events: Sender<Event>) -> Self {
        let shared = Arc::new(Shared::default());
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("rap-settings".into())
            .spawn(move || {
                loop {
                    let (settings, flushes) = {
                        let mut pending = worker.lock();
                        while pending.settings.is_none() && !pending.closed {
                            pending = worker
                                .ready
                                .wait(pending)
                                .unwrap_or_else(|e| e.into_inner());
                        }
                        let Some(settings) = pending.settings.take() else {
                            break;
                        };
                        (settings, std::mem::take(&mut pending.flushes))
                    };
                    let result = settings.save(&path);
                    if flushes.is_empty() {
                        if let Err(error) = result {
                            let failure = Failure::wrap(Kind::SettingsNotSaved, &error);
                            let _ = events.try_send(Event::Error(failure));
                        }
                        continue;
                    }
                    for reply in flushes {
                        let outcome = match &result {
                            Ok(()) => Ok(()),
                            Err(error) => Err(anyhow::anyhow!("{error:#}")),
                        };
                        let _ = reply.try_send(outcome);
                    }
                }
            })
            .expect("Cannot start the settings worker");
        Self(Arc::new(Handle(shared)))
    }

    /// Never waits for the disk: replaces any value not written yet.
    pub fn save(&self, settings: Settings) {
        let shared = &self.0.0;
        shared.lock().settings = Some(settings);
        shared.ready.notify_one();
    }

    /// Writes `settings` and waits for the outcome. Call off the UI thread.
    pub fn flush(&self, settings: Settings, timeout: Duration) -> anyhow::Result<()> {
        let shared = &self.0.0;
        let (reply, outcome) = bounded(1);
        {
            let mut pending = shared.lock();
            pending.settings = Some(settings);
            pending.flushes.push(reply);
        }
        shared.ready.notify_one();
        outcome
            .recv_timeout(timeout)
            .map_err(|_| anyhow::anyhow!("Settings were not saved in time"))?
    }
}

#[cfg(test)]
#[path = "tests/settings_writer.rs"]
mod tests;
