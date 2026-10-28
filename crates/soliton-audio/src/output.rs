//! Device stream shared by the platform backends: the worker fills a
//! preallocated queue that the device callback drains.
use crate::{OutputInfo, backend};
use anyhow::Result;
use crossbeam_queue::ArrayQueue;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

/// A stalled device fails through `failed`; a flush never waits longer.
const CALLBACK_WAIT: Duration = Duration::from_millis(200);

pub(crate) struct Output {
    stream: backend::Stream,
    pub samples: Arc<ArrayQueue<[f32; 2]>>,
    pub playing: Arc<AtomicBool>,
    pub volume: Arc<AtomicU32>,
    pub played: Arc<AtomicU64>,
    /// Completed callbacks, so the worker knows when none still consumes frames.
    callbacks: Arc<AtomicU64>,
    pub failed: Arc<AtomicBool>,
    pub rate: u32,
    /// Channels carried by queued frames: mono, or stereo for any wider device,
    /// whose further channels stay silent.
    pub frame_channels: usize,
    info: OutputInfo,
}

impl Output {
    pub fn open(rate: u32, channels: usize, volume: f32, preferred: Option<&str>) -> Result<Self> {
        let callback = Callback {
            samples: Arc::new(ArrayQueue::new(rate as usize * 2)),
            playing: Arc::new(AtomicBool::new(false)),
            volume: Arc::new(AtomicU32::new(volume.clamp(0., 1.).to_bits())),
            played: Arc::new(AtomicU64::new(0)),
            callbacks: Arc::new(AtomicU64::new(0)),
        };
        let (samples, playing, volume, played, callbacks) = (
            callback.samples.clone(),
            callback.playing.clone(),
            callback.volume.clone(),
            callback.played.clone(),
            callback.callbacks.clone(),
        );
        let failed = Arc::new(AtomicBool::new(false));
        let (stream, info) = backend::open(rate, channels, preferred, callback, failed.clone())?;
        anyhow::ensure!(info.channels > 0, "Output has no channel");
        // The device may run at another rate: the queue then holds slightly
        // more or less than two seconds, which only changes buffering.
        Ok(Self {
            stream,
            samples,
            playing,
            volume,
            played,
            callbacks,
            failed,
            rate: info.sample_rate,
            frame_channels: usize::from(info.channels).min(2),
            info,
        })
    }

    /// The device actually playing the stream, when the sound server moves it.
    pub fn info(&self) -> OutputInfo {
        let mut info = self.info.clone();
        if let Some((name, label)) = self.stream.device() {
            info.device_name = name;
            info.device_label = label;
        }
        info
    }

    /// Whether the device has called back at least once.
    pub fn ran(&self) -> bool {
        self.callbacks.load(Ordering::Relaxed) > 0
    }

    pub fn position_ms(&self) -> u64 {
        self.played.load(Ordering::Relaxed) * 1000 / self.rate as u64
    }

    /// Drops queued frames and restarts the position count, keeping the stream.
    /// Consumption stays stopped until `playing` is set again.
    pub fn flush(&self) {
        flush(&self.samples, &self.playing, &self.played, &self.callbacks);
    }
}

/// Once two callbacks have completed after `playing` is cleared, the second one
/// started after the first ended and saw it cleared: no callback pops any more,
/// so no frame from before the flush can be played after it.
fn flush(
    samples: &ArrayQueue<[f32; 2]>,
    playing: &AtomicBool,
    played: &AtomicU64,
    callbacks: &AtomicU64,
) {
    playing.store(false, Ordering::SeqCst);
    let seen = callbacks.load(Ordering::SeqCst);
    let start = Instant::now();
    while callbacks.load(Ordering::SeqCst).wrapping_sub(seen) < 2 && start.elapsed() < CALLBACK_WAIT
    {
        std::thread::sleep(Duration::from_millis(1));
    }
    while samples.pop().is_some() {}
    played.store(0, Ordering::SeqCst);
}

/// State moved into the device callback.
pub(crate) struct Callback {
    samples: Arc<ArrayQueue<[f32; 2]>>,
    playing: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
    played: Arc<AtomicU64>,
    callbacks: Arc<AtomicU64>,
}

impl Callback {
    /// Callback contract: only atomics and nonblocking preallocated queue access.
    pub fn fill<T>(&self, output: &mut [T], channels: usize, convert: impl Fn(f32) -> T) {
        let active = self.playing.load(Ordering::SeqCst);
        let gain = f32::from_bits(self.volume.load(Ordering::Relaxed));
        let consumed = render(output, channels, &self.samples, active, gain, convert);
        self.played.fetch_add(consumed, Ordering::Relaxed);
        self.callbacks.fetch_add(1, Ordering::SeqCst);
    }
}

fn render<T>(
    output: &mut [T],
    channels: usize,
    frames: &ArrayQueue<[f32; 2]>,
    active: bool,
    gain: f32,
    convert: impl Fn(f32) -> T,
) -> u64 {
    let mut consumed = 0;
    for frame in output.chunks_mut(channels) {
        let values = if active {
            frames.pop().inspect(|_| consumed += 1).unwrap_or([0.; 2])
        } else {
            [0.; 2]
        };
        for (channel, sample) in frame.iter_mut().enumerate() {
            let value = values.get(channel).copied().unwrap_or(0.);
            *sample = convert((value * gain).clamp(-1., 1.));
        }
    }
    consumed
}

#[cfg(test)]
#[path = "tests/output.rs"]
mod tests;
