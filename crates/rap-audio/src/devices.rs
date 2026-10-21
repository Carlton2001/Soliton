//! Local CPAL outputs. Names are the only portable identifiers exposed by CPAL
//! 0.16; ambiguous names are deliberately not selectable.
use anyhow::{Context, Result, ensure};
use cpal::traits::{DeviceTrait, HostTrait};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDevice {
    pub name: String,
    pub selectable: bool,
}

/// Format of the stream opened by RAP, before any downstream system processing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputInfo {
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub sample_format: String,
}

pub(crate) fn list() -> Result<Vec<OutputDevice>> {
    let names = cpal::default_host()
        .output_devices()?
        .map(|device| device.name())
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(describe(names))
}

fn describe(mut names: Vec<String>) -> Vec<OutputDevice> {
    names.sort();
    let mut devices: Vec<OutputDevice> = Vec::new();
    for name in names {
        if let Some(previous) = devices.last_mut().filter(|d| d.name == name) {
            previous.selectable = false;
        } else {
            devices.push(OutputDevice {
                name,
                selectable: true,
            });
        }
    }
    devices
}

pub(crate) fn resolve(preferred: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    let Some(name) = preferred else {
        return host
            .default_output_device()
            .context("No audio output device");
    };
    let mut matching = Vec::new();
    for device in host.output_devices()? {
        if device.name().is_ok_and(|candidate| candidate == name) {
            matching.push(device);
        }
    }
    ensure!(!matching.is_empty(), "Audio output is unavailable: {name}");
    ensure!(
        matching.len() == 1,
        "Audio output name is ambiguous: {name}"
    );
    Ok(matching.pop().unwrap())
}

#[cfg(test)]
#[path = "tests/devices.rs"]
mod tests;
