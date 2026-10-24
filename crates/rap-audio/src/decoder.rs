use anyhow::{Context, Result, bail};
use rap_core::failure::{Failure, Kind};
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{Decoder, DecoderOptions},
    errors::Error,
    formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    units::Time,
};

/// Consecutive undecodable packets skipped before the file counts as unreadable.
/// An isolated broken packet only loses a few milliseconds.
const CORRUPT_RUN: usize = 8;

pub(crate) struct Reader {
    path: PathBuf,
    /// Packets skipped because they could not be decoded, since opening.
    pub skipped: u64,
    corrupt_run: usize,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    pub rate: u32,
    pub channels: usize,
    pub duration_ms: u64,
    buffer: Option<SampleBuffer<f32>>,
    primed: Option<Vec<f32>>,
    discard_frames: usize,
    time_base: Option<symphonia::core::units::TimeBase>,
}

#[cfg(test)]
#[path = "tests/decoder.rs"]
mod tests;

/// Names the file: an unclassified error means its audio cannot be read.
fn in_file(path: &Path, error: anyhow::Error) -> anyhow::Error {
    Failure::wrap(Kind::UnreadableAudio, &error)
        .about(path.display())
        .into()
}

fn unsupported(detail: String) -> anyhow::Error {
    Failure::with(Kind::UnsupportedAudio, detail).into()
}

impl Reader {
    pub fn open(path: &Path) -> Result<Self> {
        Self::probe(path).map_err(|error| in_file(path, error))
    }

    fn probe(path: &Path) -> Result<Self> {
        let source = MediaSourceStream::new(Box::new(File::open(path)?), Default::default());
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            hint.with_extension(extension);
        }
        let probed = symphonia::default::get_probe().format(
            &hint,
            source,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )?;
        let format = probed.format;
        let track = format.default_track().context("No audio track")?;
        let parameters = &track.codec_params;
        let frames = parameters.n_frames;
        let decoder =
            symphonia::default::get_codecs().make(parameters, &DecoderOptions::default())?;
        let time_base = parameters.time_base;
        let mut reader = Self {
            path: path.to_path_buf(),
            skipped: 0,
            corrupt_run: 0,
            track_id: track.id,
            format,
            decoder,
            rate: 0,
            channels: 0,
            duration_ms: 0,
            buffer: None,
            primed: None,
            discard_frames: 0,
            time_base,
        };
        // AAC and some containers reveal their channel layout only after decoding
        // the first packet. Prime once before opening the output device.
        reader.primed = Some(reader.decode()?.context("No decoded audio samples")?);
        reader.duration_ms = frames.map_or(0, |frames| {
            if let Some(base) = time_base {
                let time = base.calc_time(frames);
                time.seconds * 1000 + (time.frac * 1000.) as u64
            } else {
                frames.saturating_mul(1000) / reader.rate as u64
            }
        });
        Ok(reader)
    }

    pub fn next(&mut self) -> Result<Option<Vec<f32>>> {
        if let Some(samples) = self.primed.take() {
            return Ok(Some(samples));
        }
        self.decode().map_err(|error| in_file(&self.path, error))
    }

    fn decode(&mut self) -> Result<Option<Vec<f32>>> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(Error::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                Err(error) => {
                    self.skip(error)?;
                    continue;
                }
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(error) => {
                    self.skip(error)?;
                    continue;
                }
            };
            self.corrupt_run = 0;
            if self.rate == 0 {
                self.rate = decoded.spec().rate;
                self.channels = decoded.spec().channels.count();
                if !(1..=2).contains(&self.channels) {
                    return Err(unsupported(format!("{} ch", self.channels)));
                }
                if !(8000..=384000).contains(&self.rate) {
                    return Err(unsupported(format!("{} Hz", self.rate)));
                }
            }
            let (rate, channels) = (decoded.spec().rate, decoded.spec().channels.count());
            if rate != self.rate || channels != self.channels {
                return Err(unsupported(format!(
                    "{} Hz, {} ch → {rate} Hz, {channels} ch",
                    self.rate, self.channels
                )));
            }
            if decoded.capacity() > 384000 * 2 {
                bail!("Decoded packet is too large");
            }
            if self
                .buffer
                .as_ref()
                .is_none_or(|b| b.capacity() < decoded.capacity() * self.channels)
            {
                self.buffer = Some(SampleBuffer::new(
                    decoded.capacity() as u64,
                    *decoded.spec(),
                ));
            }
            let buffer = self.buffer.as_mut().unwrap();
            buffer.copy_interleaved_ref(decoded);
            let skip = self
                .discard_frames
                .min(buffer.samples().len() / self.channels);
            self.discard_frames -= skip;
            let samples = &buffer.samples()[skip * self.channels..];
            if samples.is_empty() {
                continue;
            }
            return Ok(Some(samples.to_vec()));
        }
    }

    /// Counts a corrupted packet as skipped; other errors, and a run of corrupted
    /// packets, stop decoding.
    fn skip(&mut self, error: Error) -> Result<()> {
        if !matches!(error, Error::DecodeError(_)) || self.corrupt_run >= CORRUPT_RUN {
            return Err(error.into());
        }
        self.corrupt_run += 1;
        self.skipped += 1;
        Ok(())
    }

    pub fn seek(&mut self, milliseconds: u64) -> Result<u64> {
        self.seek_to(milliseconds)
            .map_err(|error| in_file(&self.path, error))
    }

    fn seek_to(&mut self, milliseconds: u64) -> Result<u64> {
        let position = if self.duration_ms > 0 {
            milliseconds.min(self.duration_ms)
        } else {
            milliseconds
        };
        let seeked = self.format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: Time::new(position / 1000, (position % 1000) as f64 / 1000.),
                track_id: Some(self.track_id),
            },
        )?;
        self.decoder.reset();
        self.primed = None;
        self.corrupt_run = 0;
        // Discard samples before required_ts after the decoder's preceding seek point.
        let delta = seeked.required_ts.saturating_sub(seeked.actual_ts);
        self.discard_frames = if let Some(base) = self.time_base {
            let time = base.calc_time(delta);
            (time.seconds * self.rate as u64) as usize + (time.frac * self.rate as f64) as usize
        } else {
            delta as usize
        };
        Ok(position)
    }
}
