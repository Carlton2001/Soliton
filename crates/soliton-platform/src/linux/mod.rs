//! Session bus and XDG files are owned by one worker; the UI only enqueues.
//! Soliton does not register a sign-in launch on Linux.
mod appearance;
pub(crate) mod documents;
pub(crate) mod gpu;
mod launcher_icon;
mod mpris;

use crate::{DesktopEvent, MediaState};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use soliton_core::config::{AppIcon, Language};
use std::thread::JoinHandle;

type Update = (MediaState, Language, AppIcon);

pub(super) struct Integration {
    /// Dropping it ends the worker, which sleeps until an update arrives.
    updates: Option<Sender<Update>>,
    stale: Receiver<Update>,
    worker: Option<JoinHandle<()>>,
}

impl Integration {
    pub fn start(icon: AppIcon) -> Result<(Self, Receiver<DesktopEvent>)> {
        let (events, receiver) = bounded(64);
        let (updates, latest) = bounded(1);
        let stale = latest.clone();
        let worker = std::thread::Builder::new()
            .name("soliton-desktop".into())
            .spawn(move || run(icon, &events, latest))?;
        Ok((
            Self {
                updates: Some(updates),
                stale,
                worker: Some(worker),
            },
            receiver,
        ))
    }

    pub fn publish(&self, state: MediaState, language: Language, icon: AppIcon) {
        let Some(updates) = &self.updates else {
            return;
        };
        if let Err(crossbeam_channel::TrySendError::Full(update)) =
            updates.try_send((state, language, icon))
        {
            let _ = self.stale.try_recv();
            let _ = updates.try_send(update);
        }
    }

    pub fn join(&mut self) {
        self.updates = None;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run(icon: AppIcon, events: &Sender<DesktopEvent>, updates: Receiver<Update>) {
    let warn = |error: String| {
        let _ = events.try_send(DesktopEvent::Warning(error));
    };
    let mut launcher = launcher_icon::LauncherIcon::default();
    if let Err(error) = launcher.apply(icon) {
        warn(format!("Launcher icon: {error:#}"));
    }
    // Without a session bus, playback works; only the system media controls are lost.
    let mut media = match mpris::Mpris::start(events.clone()) {
        Ok(media) => Some(media),
        Err(error) => {
            warn(format!("MPRIS: {error:#}"));
            None
        }
    };
    appearance::watch_accent(events.clone());
    // The other receiver, kept to drop stale updates, never sends: the
    // channel closes once the integration drops its sender.
    while let Ok((state, _, icon)) = updates.recv() {
        if let Err(error) = launcher.apply(icon) {
            warn(format!("Launcher icon: {error:#}"));
        }
        let error = media.as_mut().and_then(|media| media.update(state).err());
        if let Some(error) = error {
            warn(format!("MPRIS: {error:#}"));
            // A broken bus must not flood the UI on every position tick.
            media = None;
        }
    }
}
