//! Portable contracts for native integration. No native types leave this crate.
use anyhow::Result;
use crossbeam_channel::Receiver;
use raw_window_handle::HasWindowHandle;
use soliton_core::config::{AppIcon, Language};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaCommand {
    Play,
    Pause,
    Toggle,
    Stop,
    Previous,
    Next,
    Seek(u64),
}

#[derive(Debug)]
pub enum DesktopEvent {
    Media(MediaCommand),
    /// Theme-aware Windows accent RGB values: light, dark.
    SystemAccent([u32; 2]),
    Show,
    Quit,
    Autostart(Result<bool>),
    Warning(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransportState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaTrack {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Already extracted artwork in Soliton's cache, never an audio source.
    pub cover: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MediaState {
    pub track: Option<MediaTrack>,
    pub state: TransportState,
    pub position_ms: u64,
    pub duration_ms: u64,
    /// Portable RGB palette supplied by the application's design system.
    pub preview: PreviewColors,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewColors {
    pub background: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    /// Opaque progress rail color, composited by the application design system.
    pub progress_background: u32,
}

impl Default for PreviewColors {
    fn default() -> Self {
        Self {
            background: 0x191d24,
            text: 0xeef2f7,
            muted: 0x9ba8b8,
            accent: 0x68d4b3,
            progress_background: 0x40474f,
        }
    }
}

pub struct Desktop {
    pub events: Receiver<DesktopEvent>,
    #[cfg(target_os = "windows")]
    native: crate::windows::Integration,
    #[cfg(target_os = "linux")]
    native: crate::linux::Integration,
}

impl Desktop {
    /// Some native integration exists; each capability is reported separately.
    pub const SUPPORTED: bool = cfg!(any(target_os = "windows", target_os = "linux"));
    /// A notification area icon can restore a hidden window, so closing may hide.
    pub const HIDES: bool = cfg!(target_os = "windows");
    /// Sign-in launch can be registered by the user; not offered on Linux,
    /// where GNOME has no tray and the window would open at every sign-in.
    pub const AUTOSTART: bool = cfg!(target_os = "windows");

    /// The caller must drop the integration before destroying the borrowed window.
    pub fn start(window: &impl HasWindowHandle, language: Language, icon: AppIcon) -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            let handle = window
                .window_handle()
                .map_err(|error| anyhow::anyhow!(error.to_string()))?
                .as_raw();
            let (native, events) = crate::windows::Integration::start(handle, language, icon)?;
            Ok(Self { native, events })
        }
        #[cfg(target_os = "linux")]
        {
            // The session bus and XDG files need no window handle, which GPUI's
            // X11 backend does not provide.
            let _ = (window, language);
            let (native, events) = crate::linux::Integration::start(icon)?;
            Ok(Self { native, events })
        }
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            let _ = (window, language, icon);
            anyhow::bail!("Desktop integration is not implemented on this platform")
        }
    }

    pub fn publish(&self, state: MediaState, language: Language, icon: AppIcon) {
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        self.native.publish(state, language, icon);
        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        let _ = (state, language, icon);
    }

    pub fn can_hide(&self) -> bool {
        #[cfg(target_os = "windows")]
        return self.native.can_hide();
        #[cfg(not(target_os = "windows"))]
        false
    }

    /// Returns false when no working tray icon can restore the window.
    pub fn hide(&self) -> bool {
        #[cfg(target_os = "windows")]
        return self.native.hide();
        #[cfg(not(target_os = "windows"))]
        false
    }

    /// Restores a hidden window. Without [`Self::HIDES`], the caller activates
    /// its own window through the toolkit instead.
    pub fn show(&self) {
        #[cfg(target_os = "windows")]
        self.native.show();
    }

    pub fn set_autostart(&self, enabled: bool) -> Result<()> {
        #[cfg(target_os = "windows")]
        return self.native.set_autostart(enabled);
        #[cfg(not(target_os = "windows"))]
        {
            let _ = enabled;
            anyhow::bail!("Autostart is not implemented on this platform")
        }
    }

    /// Waits for native cleanup. Call on a background executor during app quit.
    pub fn shutdown(self) {
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        {
            let mut native = self.native;
            native.join();
        }
    }
}
