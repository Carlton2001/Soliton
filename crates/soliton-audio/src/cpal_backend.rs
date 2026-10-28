//! CPAL outputs, outside Linux. Names are the only portable identifiers exposed
//! by CPAL 0.16.
use crate::{OutputDevice, OutputInfo, devices::describe, output::Callback};
use anyhow::{Result, bail};
use cpal::{
    FromSample, SampleFormat, SizedSample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use soliton_core::failure::{Failure, Kind};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

/// The enumeration thread stops once the engine no longer reads its results.
pub(crate) struct Outputs;

/// Device enumeration can involve slow drivers. It never holds up decoding.
pub(crate) fn watch_outputs(
    discovered: Sender<Result<Vec<OutputDevice>>>,
    refresh: Receiver<()>,
    watched: Arc<AtomicBool>,
) -> Result<Outputs> {
    std::thread::Builder::new()
        .name("soliton-outputs".into())
        .spawn(move || {
            loop {
                if discovered.send(list()).is_err() {
                    break;
                }
                let interval = if watched.load(Ordering::Relaxed) {
                    Duration::from_secs(60)
                } else {
                    Duration::from_secs(5)
                };
                match refresh.recv_timeout(interval) {
                    Ok(()) | Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        })?;
    Ok(Outputs)
}

fn list() -> Result<Vec<OutputDevice>> {
    let devices = cpal::default_host()
        .output_devices()?
        .map(|device| {
            device
                .name()
                .map(|name| OutputDevice::new(name.clone(), name, false))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(describe(devices))
}

pub(crate) fn check(name: &str) -> Result<()> {
    resolve(Some(name)).map(|_| ())
}

fn resolve(preferred: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    let Some(name) = preferred else {
        return Ok(host
            .default_output_device()
            .ok_or(Failure::new(Kind::NoOutputDevice))?);
    };
    let mut matching = Vec::new();
    for device in host.output_devices()? {
        if device.name().is_ok_and(|candidate| candidate == name) {
            matching.push(device);
        }
    }
    if matching.len() != 1 {
        let kind = if matching.is_empty() {
            Kind::OutputUnavailable
        } else {
            Kind::OutputAmbiguous
        };
        return Err(Failure::with(kind, name).into());
    }
    Ok(matching.pop().unwrap())
}

pub(crate) struct Stream {
    _stream: cpal::Stream,
}

impl Stream {
    /// CPAL plays on the opened device until the stream fails.
    pub fn device(&self) -> Option<(String, String)> {
        None
    }
}

pub(crate) fn open(
    rate: u32,
    channels: usize,
    preferred: Option<&str>,
    callback: Callback,
    failed: Arc<AtomicBool>,
) -> Result<(Stream, OutputInfo)> {
    let device = resolve(preferred)?;
    let default = device.default_output_config()?;
    let supported = device.supported_output_configs()?.find(|c| {
        c.channels() as usize == channels
            && c.min_sample_rate().0 <= rate
            && c.max_sample_rate().0 >= rate
            && c.sample_format() == SampleFormat::F32
    });
    let config = supported
        .map(|c| c.with_sample_rate(cpal::SampleRate(rate)))
        .unwrap_or(default);
    // Shared-mode WASAPI only accepts the mix format's channel count, which
    // is 6 or 8 for surround speakers: stereo goes to front left and right.
    let name = device.name()?;
    let info = OutputInfo {
        device_name: name.clone(),
        device_label: name,
        sample_rate: config.sample_rate().0,
        channels: config.channels(),
        sample_format: format!("{:?}", config.sample_format()),
    };
    let stream_config = config.config();
    let stream = match config.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, &stream_config, callback, failed)?,
        SampleFormat::F64 => build::<f64>(&device, &stream_config, callback, failed)?,
        SampleFormat::I16 => build::<i16>(&device, &stream_config, callback, failed)?,
        SampleFormat::I32 => build::<i32>(&device, &stream_config, callback, failed)?,
        SampleFormat::U16 => build::<u16>(&device, &stream_config, callback, failed)?,
        format => bail!("Unsupported output sample format: {format:?}"),
    };
    stream.play()?;
    Ok((Stream { _stream: stream }, info))
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    callback: Callback,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = usize::from(config.channels);
    Ok(device.build_output_stream(
        config,
        move |output: &mut [T], _| callback.fill(output, channels, T::from_sample),
        move |_| failed.store(true, Ordering::Relaxed),
        None,
    )?)
}
