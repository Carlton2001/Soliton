//! Platform boundaries. Native APIs must stay behind target-specific modules.

mod bluetooth;
pub use bluetooth::{Bluetooth, BluetoothDevice};

mod watch;
pub use watch::{AudioOutputs, DeviceWatch};

mod desktop;
pub use desktop::TrayAnchor;
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
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        linux::updates::download(url, limit)
    }
    #[cfg(not(any(
        target_os = "windows",
        all(target_os = "linux", target_arch = "x86_64")
    )))]
    {
        let _ = (url, limit);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Update checks are unavailable on this platform",
        ))
    }
}

/// The host path of a folder chosen through a sandbox portal, when readable.
/// Blocks on the session bus under Linux: call from a worker.
pub fn host_path(path: std::path::PathBuf) -> std::path::PathBuf {
    #[cfg(target_os = "linux")]
    {
        linux::documents::host_path(path)
    }
    #[cfg(not(target_os = "linux"))]
    {
        path
    }
}

/// Linux: prefers the Wayland compositor's GPU, honoring any explicit GPU
/// selection. Call first in `main`, before any thread starts. Returns the
/// disabled Vulkan drivers and why.
pub fn prefer_display_gpu() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        linux::gpu::prefer_display_gpu()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Initializes the graphical backend, retrying Linux Vulkan without the
/// application filter if the preferred drivers fail to initialize.
pub fn initialize_display<T>(filtered: bool, initialize: impl FnOnce() -> T) -> anyhow::Result<T> {
    #[cfg(target_os = "linux")]
    {
        linux::gpu::initialize_display(filtered, initialize)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = filtered;
        Ok(initialize())
    }
}

mod paths;
pub use paths::AppPaths;
pub mod updater;
