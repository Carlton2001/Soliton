//! Session bus and XDG files are owned by one worker; the UI only enqueues.
//! Soliton does not register a sign-in launch on Linux.
mod appearance;
pub(crate) mod documents;
mod launcher_icon;
mod mpris;

use crate::{DesktopEvent, MediaState};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use soliton_core::config::{AppIcon, Language};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

type Update = (MediaState, Language, AppIcon);

pub(super) struct Integration {
    updates: Sender<Update>,
    stale: Receiver<Update>,
    shutdown: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Integration {
    pub fn start(icon: AppIcon) -> Result<(Self, Receiver<DesktopEvent>)> {
        let (events, receiver) = bounded(64);
        let (updates, latest) = bounded(1);
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = shutdown.clone();
        let stale = latest.clone();
        let worker = std::thread::Builder::new()
            .name("soliton-desktop".into())
            .spawn(move || run(icon, &events, latest, &worker_shutdown))?;
        Ok((
            Self {
                updates,
                stale,
                shutdown,
                worker: Some(worker),
            },
            receiver,
        ))
    }

    pub fn publish(&self, state: MediaState, language: Language, icon: AppIcon) {
        if let Err(crossbeam_channel::TrySendError::Full(update)) =
            self.updates.try_send((state, language, icon))
        {
            let _ = self.stale.try_recv();
            let _ = self.updates.try_send(update);
        }
    }

    pub fn join(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Integration {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

fn run(
    icon: AppIcon,
    events: &Sender<DesktopEvent>,
    updates: Receiver<Update>,
    shutdown: &AtomicBool,
) {
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
    let mut accent = appearance::AccentWatcher::new();
    while !shutdown.load(Ordering::Acquire) {
        if let Some(accent) = &mut accent {
            accent.poll(events);
        }
        if let Ok((state, _, icon)) = updates.recv_timeout(Duration::from_millis(100)) {
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
}
