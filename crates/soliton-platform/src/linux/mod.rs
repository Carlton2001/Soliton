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
use std::{
    thread::JoinHandle,
    time::{Duration, Instant},
};

/// A lost MPRIS connection is restarted on an update at most this often.
const MEDIA_RETRY: Duration = Duration::from_secs(30);

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
    let mut media = Media {
        mpris: None,
        events: events.clone(),
        retry_at: Instant::now(),
    };
    if let Err(error) = media.restart() {
        warn(format!("MPRIS: {error:#}"));
    }
    appearance::watch_accent(events.clone());
    // The other receiver, kept to drop stale updates, never sends: the
    // channel closes once the integration drops its sender.
    while let Ok((state, _, icon)) = updates.recv() {
        if let Err(error) = launcher.apply(icon) {
            warn(format!("Launcher icon: {error:#}"));
        }
        if let Some(error) = media.update(state) {
            warn(format!("MPRIS: {error:#}"));
        }
    }
}

/// System media controls that come back after a bus failure.
struct Media {
    mpris: Option<mpris::Mpris>,
    events: Sender<DesktopEvent>,
    retry_at: Instant,
}

impl Media {
    fn restart(&mut self) -> Result<()> {
        self.retry_at = Instant::now() + MEDIA_RETRY;
        self.mpris = Some(mpris::Mpris::start(self.events.clone())?);
        Ok(())
    }

    /// Publishes `state`, restarting a lost connection when due. Returns the
    /// error that lost it; failed restarts stay silent, so a broken bus never
    /// floods the interface on every position tick.
    fn update(&mut self, state: MediaState) -> Option<anyhow::Error> {
        if self.mpris.is_none() && (Instant::now() < self.retry_at || self.restart().is_err()) {
            return None;
        }
        let error = self.mpris.as_mut()?.update(state).err()?;
        self.mpris = None;
        self.retry_at = Instant::now() + MEDIA_RETRY;
        Some(error)
    }
}
