//! Native device change notifications, so callers query only after a change.
use anyhow::Result;
use crossbeam_channel::Receiver;

/// Signals device changes until dropped.
pub struct DeviceWatch {
    #[cfg(target_os = "windows")]
    _native: crate::windows::device_watch::Watch,
    /// At most one pending signal; read it, then run the matching query.
    pub changes: Receiver<()>,
}

#[cfg(target_os = "windows")]
pub(crate) fn start(
    sources: &'static [crate::windows::device_watch::Source],
) -> Result<DeviceWatch> {
    let (native, changes) = crate::windows::device_watch::watch(sources)?;
    Ok(DeviceWatch {
        _native: native,
        changes,
    })
}

/// Audio outputs known to the system.
pub struct AudioOutputs;

impl AudioOutputs {
    /// Signals added, removed, enabled or disabled audio outputs.
    pub fn watch() -> Result<DeviceWatch> {
        #[cfg(target_os = "windows")]
        return start(&[crate::windows::device_watch::Source::AudioOutputs]);
        #[cfg(not(target_os = "windows"))]
        anyhow::bail!("Audio output notifications are not implemented on this platform")
    }
}
