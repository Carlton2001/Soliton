//! Platform boundaries. Native APIs must stay behind target-specific modules.

mod bluetooth;
pub use bluetooth::{Bluetooth, BluetoothDevice};

mod watch;
pub use watch::{AudioOutputs, DeviceWatch};

mod desktop;
pub use desktop::{
    Desktop, DesktopEvent, MediaCommand, MediaState, MediaTrack, PreviewColors, TransportState,
};

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

/// Desktop entry, icon and D-Bus identity shared with the Linux package files.
pub const APP_ID: &str = "io.github.Carlton2001.Soliton";

#[cfg(target_os = "windows")]
mod windows_download;

/// Bounded HTTPS request. Call only from a worker, never from the UI thread.
pub fn download_https(url: &str, limit: usize) -> std::io::Result<Vec<u8>> {
    #[cfg(target_os = "windows")]
    {
        windows_download::download(url, limit)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (url, limit);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Update checks require Windows",
        ))
    }
}

mod paths;
pub use paths::AppPaths;
pub mod updater;
