//! Native ownership lives on one worker; callbacks only enqueue commands.
mod autostart;
pub(crate) mod bluetooth;
mod media;
mod taskbar;
mod tray;

use crate::{DesktopEvent, MediaState};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use rap_core::config::Language;
use raw_window_handle::RawWindowHandle;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};
use windows::Win32::{
    Foundation::HWND,
    System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    UI::WindowsAndMessaging::*,
};

type Update = (MediaState, Language);

pub(super) struct Integration {
    window: usize,
    updates: Sender<Update>,
    stale: Receiver<Update>,
    startup: Sender<bool>,
    ready: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    restore: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Integration {
    pub fn start(
        handle: RawWindowHandle,
        language: Language,
    ) -> Result<(Self, Receiver<DesktopEvent>)> {
        let RawWindowHandle::Win32(handle) = handle else {
            anyhow::bail!("Expected a Win32 window");
        };
        let window = handle.hwnd.get() as usize;
        let (events, receiver) = bounded(64);
        let (updates, latest) = bounded(1);
        let (startup, requests) = bounded(4);
        let ready = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));
        let restore = Arc::new(AtomicBool::new(false));
        let worker_ready = ready.clone();
        let worker_shutdown = shutdown.clone();
        let worker_restore = restore.clone();
        // Subclassing must happen on the UI thread that owns the main HWND.
        let taskbar_binding = match taskbar::install(HWND(window as *mut _), events.clone()) {
            Ok(binding) => Some(binding),
            Err(error) => {
                let _ = events.try_send(DesktopEvent::Warning(format!("Taskbar: {error:#}")));
                None
            }
        };
        let worker_taskbar = taskbar_binding.clone();
        let stale = latest.clone();
        let worker = std::thread::Builder::new()
            .name("rap-desktop".into())
            .spawn(move || {
                if let Err(error) = run(
                    window,
                    language,
                    &events,
                    latest,
                    requests,
                    &worker_ready,
                    &worker_shutdown,
                    &worker_restore,
                    worker_taskbar,
                ) {
                    let _ = events.try_send(DesktopEvent::Warning(format!("{error:#}")));
                }
                worker_ready.store(false, Ordering::Release);
                // A failed worker must never strand a hidden main window.
                // Restore here: there is no worker left to process a Show request.
                if !worker_shutdown.load(Ordering::Acquire) {
                    unsafe {
                        let _ = ShowWindowAsync(HWND(window as *mut _), SW_SHOW);
                    }
                }
            });
        let worker = match worker {
            Ok(worker) => worker,
            Err(error) => {
                if let Some(binding) = taskbar_binding {
                    taskbar::detach(HWND(window as *mut _), &binding);
                }
                return Err(error.into());
            }
        };
        Ok((
            Self {
                window,
                updates,
                stale,
                startup,
                ready,
                shutdown,
                restore,
                worker: Some(worker),
            },
            receiver,
        ))
    }

    pub fn publish(&self, state: MediaState, language: Language) {
        if let Err(crossbeam_channel::TrySendError::Full(update)) =
            self.updates.try_send((state, language))
        {
            let _ = self.stale.try_recv();
            let _ = self.updates.try_send(update);
        }
    }

    pub fn can_hide(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    pub fn hide(&self) -> bool {
        if !self.can_hide() {
            return false;
        }
        // SAFETY: the app borrows this HWND until the integration shuts down.
        unsafe {
            let _ = ShowWindowAsync(HWND(self.window as *mut _), SW_HIDE);
        }
        true
    }

    pub fn show(&self) {
        // Do not synchronously re-enter GPUI through WM_ACTIVATE/WM_SETFOCUS.
        self.restore.store(true, Ordering::Release);
    }

    pub fn set_autostart(&self, enabled: bool) -> Result<()> {
        self.startup
            .try_send(enabled)
            .map_err(|_| anyhow::anyhow!("Desktop command queue unavailable"))
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
    window: usize,
    language: Language,
    events: &Sender<DesktopEvent>,
    updates: Receiver<Update>,
    startup: Receiver<bool>,
    ready: &Arc<AtomicBool>,
    shutdown: &AtomicBool,
    restore: &AtomicBool,
    taskbar_binding: Option<Arc<taskbar::Shared>>,
) -> Result<()> {
    let mut tray = match tray::Tray::new(events.clone(), ready.clone(), language) {
        Ok(tray) => Some(tray),
        Err(error) => {
            let _ = events.try_send(DesktopEvent::Warning(format!("Tray: {error:#}")));
            None
        }
    };
    let initialized = unsafe { RoInitialize(RO_INIT_MULTITHREADED).is_ok() };
    let mut media = if initialized {
        match media::Media::new(HWND(window as *mut _), events.clone()) {
            Ok(media) => Some(media),
            Err(error) => {
                let _ = events.try_send(DesktopEvent::Warning(format!("SMTC: {error:#}")));
                None
            }
        }
    } else {
        let _ = events.try_send(DesktopEvent::Warning(
            "Cannot initialize Windows Runtime".into(),
        ));
        None
    };
    let mut taskbar = if initialized {
        taskbar_binding.as_ref().and_then(|shared| {
            match taskbar::Taskbar::new(HWND(window as *mut _), shared.clone()) {
                Ok(taskbar) => Some(taskbar),
                Err(error) => {
                    let _ = events.try_send(DesktopEvent::Warning(format!("Taskbar: {error:#}")));
                    None
                }
            }
        })
    } else {
        None
    };
    if taskbar.is_none()
        && let Some(binding) = &taskbar_binding
    {
        taskbar::detach(HWND(window as *mut _), binding);
    }
    let _ = events.try_send(DesktopEvent::Autostart(autostart::enabled()));
    while !shutdown.load(Ordering::Acquire) {
        if restore.swap(false, Ordering::AcqRel) {
            unsafe {
                let window = HWND(window as *mut _);
                let _ = ShowWindowAsync(
                    window,
                    if IsIconic(window).as_bool() {
                        SW_RESTORE
                    } else {
                        SW_SHOW
                    },
                );
                let _ = SetForegroundWindow(window);
            }
        }
        if let Ok((state, language)) = updates.recv_timeout(Duration::from_millis(40)) {
            if let Some(tray) = &mut tray {
                tray.update(state.state, language);
            }
            let error = taskbar
                .as_mut()
                .and_then(|taskbar| taskbar.update(&state, language).err());
            if let Some(error) = error {
                let _ = events.try_send(DesktopEvent::Warning(format!("Taskbar: {error:#}")));
                taskbar = None;
                if let Some(binding) = &taskbar_binding {
                    taskbar::detach(HWND(window as *mut _), binding);
                }
            }
            let error = media.as_mut().and_then(|media| media.update(&state).err());
            if let Some(error) = error {
                let _ = events.try_send(DesktopEvent::Warning(format!("SMTC: {error:#}")));
                // A broken COM service must not flood the UI on every position tick.
                media = None;
            }
        }
        for enabled in startup.try_iter() {
            let result = autostart::set(enabled).and_then(|()| autostart::enabled());
            let _ = events.try_send(DesktopEvent::Autostart(result));
        }
        // A real top-level hidden window receives TaskbarCreated after Explorer restarts.
        unsafe {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
    drop(media);
    drop(taskbar);
    if let Some(binding) = taskbar_binding {
        taskbar::detach(HWND(window as *mut _), &binding);
    }
    drop(tray);
    if initialized {
        unsafe {
            RoUninitialize();
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/desktop.rs"]
mod tests;
