use anyhow::{Context, Result, bail};
use std::{fs::File, path::Path};
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

pub(crate) struct Reader {
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

impl Reader {
    pub fn open(path: &Path) -> Result<Self> {
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
        reader.primed = Some(reader.next()?.context("No decoded audio samples")?);
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
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(Error::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                Err(error) => return Err(error.into()),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoded = match self.decoder.decode(&packet) {
                Ok(decoded) => decoded,
                // A broken packet is reported instead of silently hiding corruption.
                Err(error) => return Err(error.into()),
            };
            if self.rate == 0 {
                self.rate = decoded.spec().rate;
                self.channels = decoded.spec().channels.count();
                anyhow::ensure!(
                    (1..=2).contains(&self.channels),
                    "Only mono and stereo are supported in this milestone"
                );
                anyhow::ensure!(
                    (8000..=384000).contains(&self.rate),
                    "Unsupported sample rate"
                );
            }
            anyhow::ensure!(
                decoded.spec().rate == self.rate
                    && decoded.spec().channels.count() == self.channels,
                "Audio format changed midstream"
            );
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

    pub fn seek(&mut self, milliseconds: u64) -> Result<u64> {
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
