//! Local outputs listed by the platform backend: CPAL devices, or PipeWire
//! sinks on Linux. Ambiguous identifiers are deliberately not selectable.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDevice {
    /// Persisted identifier: the CPAL device name, or the PipeWire node name.
    pub name: String,
    /// Shown to the user; the name itself when the backend has nothing better.
    pub label: String,
    pub bluetooth: bool,
    pub selectable: bool,
}

impl OutputDevice {
    pub(crate) fn new(name: String, label: String, bluetooth: bool) -> Self {
        Self {
            name,
            label,
            bluetooth,
            selectable: true,
        }
    }
}

/// Format of the stream opened by Soliton, before any downstream system processing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputInfo {
    /// Identifier of the device playing the stream, as in [`OutputDevice::name`];
    /// empty until the sound server has linked a stream that follows its default.
    pub device_name: String,
    pub device_label: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: String,
}

/// Sorted by label; a name shared by several devices identifies none of them.
pub(crate) fn describe(mut devices: Vec<OutputDevice>) -> Vec<OutputDevice> {
    devices.sort_by(|a, b| a.label.cmp(&b.label).then_with(|| a.name.cmp(&b.name)));
    for index in 0..devices.len() {
        let name = &devices[index].name;
        if devices.iter().filter(|other| &other.name == name).count() > 1 {
            devices[index].selectable = false;
        }
    }
    let mut seen = Vec::new();
    devices.retain(|device| {
        let first = !seen.contains(&device.name);
        seen.push(device.name.clone());
        first
    });
    devices
}

#[cfg(test)]
#[path = "tests/devices.rs"]
mod tests;
