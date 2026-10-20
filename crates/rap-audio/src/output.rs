use anyhow::{Context, Result, bail};
use cpal::{
    FromSample, SampleFormat, SizedSample,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use crossbeam_queue::ArrayQueue;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};

pub(crate) struct Output {
    _stream: cpal::Stream,
    pub samples: Arc<ArrayQueue<[f32; 2]>>,
    pub playing: Arc<AtomicBool>,
    pub volume: Arc<AtomicU32>,
    pub played: Arc<AtomicU64>,
    pub failed: Arc<AtomicBool>,
    pub rate: u32,
    pub channels: u16,
}

impl Output {
    pub fn open(rate: u32, channels: usize, volume: f32) -> Result<Self> {
        let device = cpal::default_host()
            .default_output_device()
            .context("No audio output device")?;
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
        anyhow::ensure!(
            (1..=2).contains(&config.channels()),
            "Output must be configured as mono or stereo"
        );
        let rate = config.sample_rate().0;
        let channels = config.channels();
        let samples = Arc::new(ArrayQueue::new(rate as usize * 2));
        let playing = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(AtomicU32::new(volume.clamp(0., 1.).to_bits()));
        let played = Arc::new(AtomicU64::new(0));
        let failed = Arc::new(AtomicBool::new(false));
        let stream_config = config.config();
        macro_rules! build {
            ($sample:ty) => {
                build::<$sample>(
                    &device,
                    &stream_config,
                    samples.clone(),
                    playing.clone(),
                    volume.clone(),
                    played.clone(),
                    failed.clone(),
                )?
            };
        }
        let stream = match config.sample_format() {
            SampleFormat::F32 => build!(f32),
            SampleFormat::F64 => build!(f64),
            SampleFormat::I16 => build!(i16),
            SampleFormat::I32 => build!(i32),
            SampleFormat::U16 => build!(u16),
            format => bail!("Unsupported output sample format: {format:?}"),
        };
        stream.play()?;
        Ok(Self {
            _stream: stream,
            samples,
            playing,
            volume,
            played,
            failed,
            rate,
            channels,
        })
    }
    pub fn position_ms(&self) -> u64 {
        self.played.load(Ordering::Relaxed) * 1000 / self.rate as u64
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: Arc<ArrayQueue<[f32; 2]>>,
    playing: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    played: Arc<AtomicU64>,
    failed: Arc<AtomicBool>,
) -> Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    Ok(device.build_output_stream(
        config,
        move |output: &mut [T], _| {
            // Callback contract: only atomics and nonblocking preallocated queue access.
            let active = playing.load(Ordering::Relaxed);
            let gain = f32::from_bits(volume.load(Ordering::Relaxed));
            let consumed = render(output, channels, &samples, active, gain);
            played.fetch_add(consumed, Ordering::Relaxed);
        },
        move |_| {
            failed.store(true, Ordering::Relaxed);
        },
        None,
    )?)
}

fn render<T: SizedSample + FromSample<f32>>(
    output: &mut [T],
    channels: usize,
    frames: &ArrayQueue<[f32; 2]>,
    active: bool,
    gain: f32,
) -> u64 {
    let mut consumed = 0;
    for frame in output.chunks_mut(channels) {
        let values = if active {
            frames.pop().inspect(|_| consumed += 1).unwrap_or([0.; 2])
        } else {
            [0.; 2]
        };
        for (channel, sample) in frame.iter_mut().enumerate() {
            *sample = T::from_sample((values[channel] * gain).clamp(-1., 1.));
        }
    }
    consumed
}

#[cfg(test)]
#[path = "tests/output.rs"]
mod tests;
