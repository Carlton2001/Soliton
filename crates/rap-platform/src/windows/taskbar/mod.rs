//! Windows taskbar thumbnails and their native transport toolbar.
mod bitmap;
mod hook;
mod icons;
mod render;

use crate::{DesktopEvent, MediaCommand, MediaState, TransportState};
use anyhow::Result;
use crossbeam_channel::Sender;
use icons::Icon;
use rap_core::config::Language;
use render::Renderer;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::HWND,
    Graphics::Dwm::DwmInvalidateIconicBitmaps,
    System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance},
    UI::Shell::*,
};

pub(super) const PREVIOUS: u32 = 0x5240;
pub(super) const TOGGLE: u32 = PREVIOUS + 1;
pub(super) const NEXT: u32 = PREVIOUS + 2;
pub(super) const STOP: u32 = PREVIOUS + 3;

pub(super) struct Shared {
    frame: Mutex<Option<Arc<bitmap::Frame>>>,
    events: Sender<DesktopEvent>,
    generation: AtomicU32,
    enabled: AtomicBool,
    toolbar_ready: AtomicBool,
    thumbnails: AtomicU32,
    created: u32,
    detach: u32,
}

pub(super) fn install(window: HWND, events: Sender<DesktopEvent>) -> Result<Arc<Shared>> {
    hook::install(window, events)
}

pub(super) fn detach(window: HWND, shared: &Shared) {
    hook::detach(window, shared);
}

pub(super) struct Taskbar {
    window: HWND,
    shared: Arc<Shared>,
    api: ITaskbarList3,
    icons: [Icon; 5],
    renderer: Renderer,
    last_frame: Option<(MediaState, Language)>,
    last_buttons: Option<(bool, bool, Language)>,
    generation: u32,
    next_attempt: Instant,
}

impl Taskbar {
    pub fn new(window: HWND, shared: Arc<Shared>) -> Result<Self> {
        let api: ITaskbarList3 =
            unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)? };
        unsafe {
            api.HrInit()?;
        }
        let icons = [
            Icon::new(include_bytes!("../../../../../assets/icons/skip-back.svg"))?,
            Icon::new(include_bytes!("../../../../../assets/icons/play.svg"))?,
            Icon::new(include_bytes!("../../../../../assets/icons/pause.svg"))?,
            Icon::new(include_bytes!(
                "../../../../../assets/icons/skip-forward.svg"
            ))?,
            Icon::new(include_bytes!("../../../../../assets/icons/square.svg"))?,
        ];
        Ok(Self {
            window,
            shared,
            api,
            icons,
            renderer: Renderer::default(),
            last_frame: None,
            last_buttons: None,
            generation: 0,
            next_attempt: Instant::now(),
        })
    }

    pub fn update(&mut self, state: &MediaState, language: Language) -> Result<()> {
        if !self.shared.enabled.load(Ordering::Acquire) {
            return Ok(());
        }
        let mut display = state.clone();
        // Refresh at most once per second, and immediately on state/metadata/theme changes.
        display.position_ms = display.position_ms / 1000 * 1000;
        if self.last_frame.as_ref() != Some(&(display.clone(), language)) {
            let frame = Arc::new(self.renderer.render(&display, language)?);
            if let Ok(mut current) = self.shared.frame.lock() {
                *current = Some(frame);
            }
            self.last_frame = Some((display, language));
            unsafe {
                let _ = DwmInvalidateIconicBitmaps(self.window);
            }
        }
        let generation = self.shared.generation.load(Ordering::Acquire);
        if generation != self.generation {
            self.generation = generation;
            self.shared.toolbar_ready.store(false, Ordering::Release);
            self.last_buttons = None;
            self.next_attempt = Instant::now();
        }
        let enabled = state.track.is_some();
        let playing = state.state == TransportState::Playing;
        let buttons = self.buttons(enabled, playing, language);
        if !self.shared.toolbar_ready.load(Ordering::Acquire) {
            // A hidden or not-yet-shown main window has no taskbar button yet.
            if Instant::now() >= self.next_attempt {
                self.next_attempt = Instant::now() + Duration::from_millis(500);
                if unsafe {
                    self.api
                        .ThumbBarAddButtons(self.window, &buttons)
                        .or_else(|_| self.api.ThumbBarUpdateButtons(self.window, &buttons))
                }
                .is_ok()
                {
                    self.shared.toolbar_ready.store(true, Ordering::Release);
                    self.last_buttons = Some((enabled, playing, language));
                }
            }
        } else if self.last_buttons != Some((enabled, playing, language)) {
            unsafe {
                self.api.ThumbBarUpdateButtons(self.window, &buttons)?;
            }
            self.last_buttons = Some((enabled, playing, language));
        }
        Ok(())
    }

    fn buttons(&self, enabled: bool, playing: bool, language: Language) -> [THUMBBUTTON; 4] {
        let french = language == Language::French;
        let labels = [
            if french { "Précédente" } else { "Previous" },
            if playing {
                "Pause"
            } else if french {
                "Lire"
            } else {
                "Play"
            },
            if french { "Suivante" } else { "Next" },
            if french { "Arrêter" } else { "Stop" },
        ];
        let glyphs = [0, if playing { 2 } else { 1 }, 3, 4];
        std::array::from_fn(|index| {
            let mut button = THUMBBUTTON {
                dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
                iId: PREVIOUS + index as u32,
                hIcon: self.icons[glyphs[index]].0,
                dwFlags: if enabled { THBF_ENABLED } else { THBF_DISABLED },
                ..Default::default()
            };
            for (dest, unit) in button.szTip.iter_mut().zip(labels[index].encode_utf16()) {
                *dest = unit;
            }
            button
        })
    }
}

impl Drop for Taskbar {
    fn drop(&mut self) {
        if self.shared.toolbar_ready.load(Ordering::Acquire) {
            let mut buttons = self.buttons(false, false, Language::English);
            for button in &mut buttons {
                button.dwFlags = THBF_HIDDEN;
            }
            unsafe {
                let _ = self.api.ThumbBarUpdateButtons(self.window, &buttons);
            }
        }
    }
}

fn command(id: u32) -> Option<MediaCommand> {
    match id {
        PREVIOUS => Some(MediaCommand::Previous),
        TOGGLE => Some(MediaCommand::Toggle),
        NEXT => Some(MediaCommand::Next),
        STOP => Some(MediaCommand::Stop),
        _ => None,
    }
}

#[cfg(test)]
#[path = "../tests/taskbar.rs"]
mod tests;
