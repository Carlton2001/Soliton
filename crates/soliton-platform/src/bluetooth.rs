//! Pairing belongs to the OS; Soliton uses the resulting ordinary audio outputs.
use crate::DeviceWatch;
use anyhow::Result;

pub struct Bluetooth;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BluetoothDevice {
    /// Opaque OS identifier. Never infer audio routing from the friendly name.
    pub id: String,
    pub name: String,
    pub connected: bool,
    /// Audio endpoints associated by Windows container identity, not name.
    /// The caller must also check availability in the current audio engine list.
    pub outputs: Vec<String>,
}

impl Bluetooth {
    pub const SUPPORTED: bool = cfg!(target_os = "windows");

    /// Paired devices, including disconnected devices. Bounded native queries;
    /// call from a background executor, independently of audio decoding.
    pub fn devices() -> Result<Vec<BluetoothDevice>> {
        #[cfg(target_os = "windows")]
        return crate::windows::bluetooth::devices();
        #[cfg(not(target_os = "windows"))]
        anyhow::bail!("Bluetooth enumeration is not implemented on this platform")
    }

    /// Native change notifications, replacing periodic queries. Cheap to keep
    /// while the device list is visible; drop it to stop watching.
    /// Audio endpoints are watched too: they decide which devices are ready.
    pub fn watch() -> Result<DeviceWatch> {
        #[cfg(target_os = "windows")]
        {
            use crate::windows::device_watch::Source;
            crate::watch::start(&[Source::PairedBluetooth, Source::AudioOutputs])
        }
        #[cfg(not(target_os = "windows"))]
        anyhow::bail!("Bluetooth notifications are not implemented on this platform")
    }

    /// Opens the native pairing/connection settings, without changing devices or
    /// the system's default output. Call from a background executor.
    pub fn open_settings() -> Result<()> {
        #[cfg(target_os = "windows")]
        return crate::windows::bluetooth::open_settings();
        #[cfg(not(target_os = "windows"))]
        anyhow::bail!("Bluetooth settings are not implemented on this platform")
    }
}
