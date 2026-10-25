use anyhow::Result;
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::collections::VecDeque;

pub(crate) struct Converter {
    input_channels: usize,
    output_channels: usize,
    ratio: f64,
    resampler: Option<SincFixedIn<f32>>,
    pending: VecDeque<f32>,
    delay: usize,
    input_frames: usize,
    output_frames: usize,
}

impl Converter {
    pub fn new(
        input_rate: u32,
        input_channels: usize,
        output_rate: u32,
        output_channels: usize,
    ) -> Result<Self> {
        let ratio = output_rate as f64 / input_rate as f64;
        let resampler = if input_rate == output_rate {
            None
        } else {
            Some(SincFixedIn::new(
                ratio,
                1.0,
                SincInterpolationParameters {
                    sinc_len: 128,
                    f_cutoff: 0.95,
                    interpolation: SincInterpolationType::Linear,
                    oversampling_factor: 128,
                    window: WindowFunction::BlackmanHarris2,
                },
                1024,
                output_channels,
            )?)
        };
        let delay = resampler.as_ref().map_or(0, |r| r.output_delay());
        Ok(Self {
            input_channels,
            output_channels,
            ratio,
            resampler,
            pending: VecDeque::new(),
            delay,
            input_frames: 0,
            output_frames: 0,
        })
    }

    pub fn process(&mut self, samples: &[f32], finished: bool) -> Result<Vec<f32>> {
        self.input_frames += samples.len() / self.input_channels;
        let mut mapped =
            Vec::with_capacity(samples.len() * self.output_channels / self.input_channels);
        for frame in samples.chunks_exact(self.input_channels) {
            if self.output_channels == 1 {
                mapped.push(frame.iter().sum::<f32>() / frame.len() as f32);
            } else {
                for channel in 0..self.output_channels {
                    mapped.push(if self.input_channels == 1 {
                        frame[0]
                    } else {
                        frame[channel.min(self.input_channels - 1)]
                    });
                }
            }
        }
        let Some(resampler) = &mut self.resampler else {
            return Ok(mapped);
        };
        self.pending.extend(mapped);
        let mut output = Vec::new();
        let target = (self.input_frames as f64 * self.ratio).round() as usize;
        let mut flushed = 0;
        while self.pending.len() >= 1024 * self.output_channels
            || (finished && self.output_frames < target && flushed < 8)
        {
            let mut planar = vec![vec![0.; 1024]; self.output_channels];
            for frame in 0..1024 {
                for channel in &mut planar {
                    channel[frame] = self.pending.pop_front().unwrap_or(0.);
                }
            }
            let converted = resampler.process(&planar, None)?;
            for frame in 0..converted[0].len() {
                if self.delay > 0 {
                    self.delay -= 1;
                    continue;
                }
                if finished && self.output_frames >= target {
                    break;
                }
                for channel in &converted {
                    output.push(channel[frame]);
                }
                self.output_frames += 1;
            }
            if finished {
                flushed += 1;
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
#[path = "tests/resample.rs"]
mod tests;
