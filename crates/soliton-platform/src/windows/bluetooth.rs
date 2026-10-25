//! Windows owns discovery, pairing, reconnecting and Bluetooth audio codecs.
use crate::BluetoothDevice as PairedDevice;
use anyhow::{Result, ensure};
use std::time::{Duration, Instant};
use windows::{
    Devices::{
        Bluetooth::{BluetoothDevice, BluetoothLEDevice},
        Enumeration::{DeviceInformation, DeviceInformationCollection, DeviceInformationKind},
    },
    Foundation::IPropertyValue,
    Media::Devices::MediaDevice,
    Win32::{
        Foundation::RPC_E_CHANGED_MODE,
        System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL},
    },
    core::{GUID, HSTRING, Interface, RuntimeType, w},
};
use windows_collections::IIterable;
use windows_future::{AsyncStatus, IAsyncOperation};

pub(super) struct Runtime(bool);
impl Runtime {
    pub(super) fn new() -> Result<Self> {
        let status = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        match status {
            Ok(()) => Ok(Self(true)),
            Err(error) if error.code() == RPC_E_CHANGED_MODE => Ok(Self(false)),
            Err(error) => Err(error.into()),
        }
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                RoUninitialize();
            }
        }
    }
}

fn wait<T: RuntimeType + 'static>(operation: IAsyncOperation<T>) -> Result<T> {
    let deadline = Instant::now() + Duration::from_secs(8);
    while operation.Status()? == AsyncStatus::Started {
        if Instant::now() >= deadline {
            let _ = operation.Cancel();
            anyhow::bail!("Bluetooth device query timed out");
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    Ok(operation.GetResults()?)
}

fn property(device: &DeviceInformation, key: &str) -> Option<IPropertyValue> {
    device
        .Properties()
        .ok()?
        .Lookup(&HSTRING::from(key))
        .ok()?
        .cast()
        .ok()
}

struct Device {
    id: String,
    name: String,
    container: Option<GUID>,
    connected: bool,
}
struct Endpoint {
    name: String,
    container: Option<GUID>,
}

fn associate(devices: Vec<Device>, endpoints: Vec<Endpoint>) -> Vec<PairedDevice> {
    let mut result: Vec<_> = devices
        .into_iter()
        .map(|device| {
            let mut outputs: Vec<_> = endpoints
                .iter()
                .filter(|endpoint| {
                    device
                        .container
                        .is_some_and(|id| id != GUID::zeroed() && Some(id) == endpoint.container)
                })
                .map(|endpoint| endpoint.name.clone())
                .collect();
            outputs.sort();
            outputs.dedup();
            PairedDevice {
                id: device.id,
                name: device.name,
                connected: device.connected,
                outputs,
            }
        })
        .collect();
    result.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    result.dedup_by(|a, b| a.id == b.id);
    result
}

fn query(
    selector: &HSTRING,
    kind: DeviceInformationKind,
    keys: &[&str],
) -> Result<DeviceInformationCollection> {
    let properties: IIterable<HSTRING> = keys
        .iter()
        .map(|key| HSTRING::from(*key))
        .collect::<Vec<_>>()
        .into();
    wait(
        DeviceInformation::FindAllAsyncWithKindAqsFilterAndAdditionalProperties(
            selector,
            &properties,
            kind,
        )?,
    )
}

pub(crate) fn devices() -> Result<Vec<PairedDevice>> {
    let _runtime = Runtime::new()?;
    let selector = HSTRING::from(format!(
        "({}) OR ({})",
        BluetoothDevice::GetDeviceSelectorFromPairingState(true)?,
        BluetoothLEDevice::GetDeviceSelectorFromPairingState(true)?
    ));
    let paired = query(
        &selector,
        DeviceInformationKind::AssociationEndpoint,
        &[
            "System.Devices.Aep.ContainerId",
            "System.Devices.Aep.IsConnected",
        ],
    )?;
    let mut devices = Vec::new();
    for item in paired {
        devices.push(Device {
            id: item.Id()?.to_string(),
            name: item.Name()?.to_string(),
            container: property(&item, "System.Devices.Aep.ContainerId")
                .and_then(|v| v.GetGuid().ok()),
            connected: property(&item, "System.Devices.Aep.IsConnected")
                .and_then(|v| v.GetBoolean().ok())
                .unwrap_or(false),
        });
    }
    let audio = query(
        &MediaDevice::GetAudioRenderSelector()?,
        DeviceInformationKind::DeviceInterface,
        &["System.Devices.ContainerId"],
    )?;
    let mut endpoints = Vec::new();
    for item in audio {
        endpoints.push(Endpoint {
            name: item.Name()?.to_string(),
            container: property(&item, "System.Devices.ContainerId").and_then(|v| v.GetGuid().ok()),
        });
    }
    Ok(associate(devices, endpoints))
}

pub(crate) fn open_settings() -> Result<()> {
    let _runtime = Runtime::new()?;
    // Fixed URI: no command shell, executable lookup or user-provided argument.
    // Showing Settings is intentional: the user completes native pairing there.
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            w!("ms-settings:bluetooth"),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    let code = result.0 as isize;
    ensure!(
        code > 32,
        "Cannot open Bluetooth settings (Windows: {code})"
    );
    Ok(())
}

#[cfg(test)]
#[path = "tests/bluetooth.rs"]
mod tests;
