//! Native notifications replacing periodic device queries. Events only signal
//! that something changed; the caller then runs its own bounded query.
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use windows::{
    Devices::{
        Bluetooth::{BluetoothDevice, BluetoothLEDevice},
        Enumeration::{
            DeviceInformation, DeviceInformationKind, DeviceInformationUpdate, DeviceWatcher,
            DeviceWatcherStatus,
        },
    },
    Foundation::TypedEventHandler,
    Media::Devices::MediaDevice,
    core::{HSTRING, IInspectable},
};
use windows_collections::IIterable;

/// Device sets that can be watched.
#[derive(Clone, Copy)]
pub(crate) enum Source {
    /// Paired Bluetooth and Bluetooth LE devices, with their connection state.
    PairedBluetooth,
    /// Enabled audio render endpoints.
    AudioOutputs,
}

/// Stops the native watchers when dropped.
pub(crate) struct Watch {
    stop: Option<Sender<()>>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        // Closing the channel wakes the owner thread, which stops the watchers.
        self.stop.take();
    }
}

/// Starts watching on a dedicated thread owning the Windows Runtime apartment.
/// `changes` holds at most one pending signal: bursts collapse into one query.
pub(crate) fn watch(sources: &'static [Source]) -> Result<(Watch, Receiver<()>)> {
    let (changes, receiver) = bounded(1);
    let (stop, stopped) = bounded::<()>(0);
    let (started, ready) = bounded(1);
    std::thread::Builder::new()
        .name("rap-device-watch".into())
        .spawn(move || {
            // The runtime stays initialized on this thread while watchers live.
            let watchers = super::bluetooth::Runtime::new()
                .and_then(|runtime| start(sources, &changes).map(|watchers| (runtime, watchers)));
            match watchers {
                Ok((_runtime, watchers)) => {
                    let _ = started.send(Ok(()));
                    let _ = stopped.recv();
                    for watcher in watchers {
                        if matches!(
                            watcher.Status(),
                            Ok(DeviceWatcherStatus::Started
                                | DeviceWatcherStatus::EnumerationCompleted)
                        ) {
                            let _ = watcher.Stop();
                        }
                    }
                }
                Err(error) => {
                    let _ = started.send(Err(error));
                }
            }
        })?;
    ready.recv()??;
    Ok((Watch { stop: Some(stop) }, receiver))
}

fn start(sources: &[Source], changes: &Sender<()>) -> Result<Vec<DeviceWatcher>> {
    let mut watchers = Vec::new();
    for source in sources {
        watchers.push(match source {
            Source::PairedBluetooth => watcher(
                &HSTRING::from(format!(
                    "({}) OR ({})",
                    BluetoothDevice::GetDeviceSelectorFromPairingState(true)?,
                    BluetoothLEDevice::GetDeviceSelectorFromPairingState(true)?
                )),
                DeviceInformationKind::AssociationEndpoint,
                &["System.Devices.Aep.IsConnected"],
                changes,
            )?,
            Source::AudioOutputs => watcher(
                &MediaDevice::GetAudioRenderSelector()?,
                DeviceInformationKind::DeviceInterface,
                &[],
                changes,
            )?,
        });
    }
    for watcher in &watchers {
        watcher.Start()?;
    }
    Ok(watchers)
}

fn watcher(
    selector: &HSTRING,
    kind: DeviceInformationKind,
    keys: &[&str],
    changes: &Sender<()>,
) -> Result<DeviceWatcher> {
    let properties: IIterable<HSTRING> = keys
        .iter()
        .map(|key| HSTRING::from(*key))
        .collect::<Vec<_>>()
        .into();
    let watcher = DeviceInformation::CreateWatcherWithKindAqsFilterAndAdditionalProperties(
        selector,
        &properties,
        kind,
    )?;
    // The initial enumeration reports every existing device; the caller
    // queries once on its own, so only later changes are signalled.
    let enumerated = Arc::new(AtomicBool::new(false));
    let signal = {
        let (changes, enumerated) = (changes.clone(), enumerated.clone());
        move || {
            if enumerated.load(Ordering::Acquire) {
                let _ = changes.try_send(());
            }
        }
    };
    let added = signal.clone();
    watcher.Added(&TypedEventHandler::<DeviceWatcher, DeviceInformation>::new(
        move |_, _| {
            added();
            Ok(())
        },
    ))?;
    let updated = signal.clone();
    watcher.Updated(
        &TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(move |_, _| {
            updated();
            Ok(())
        }),
    )?;
    watcher.Removed(
        &TypedEventHandler::<DeviceWatcher, DeviceInformationUpdate>::new(move |_, _| {
            signal();
            Ok(())
        }),
    )?;
    watcher.EnumerationCompleted(&TypedEventHandler::<DeviceWatcher, IInspectable>::new(
        move |_, _| {
            enumerated.store(true, Ordering::Release);
            Ok(())
        },
    ))?;
    Ok(watcher)
}

#[cfg(test)]
#[path = "tests/device_watch.rs"]
mod tests;
