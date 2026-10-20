use crate::{
    Command, Playback, PlaybackState, decoder::Reader, output::Output, queue::Queue,
    resample::Converter,
};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub struct Engine {
    commands: Sender<Command>,
    pub events: Receiver<Playback>,
    shutdown: Arc<AtomicBool>,
}

impl Engine {
    pub fn start(volume: f32) -> Result<Self> {
        let (commands, receiver) = bounded(32);
        let (events, snapshots) = bounded(8);
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = shutdown.clone();
        std::thread::Builder::new()
            .name("rap-audio".into())
            .spawn(move || {
                let mut worker = Worker {
                    queue: Queue::default(),
                    playback: Playback {
                        volume: volume.clamp(0., 1.),
                        ..Default::default()
                    },
                    session: None,
                };
                let mut last = Instant::now() - Duration::from_secs(1);
                while !worker_shutdown.load(Ordering::Relaxed) {
                    let hungry = worker.session.as_ref().is_some_and(|s| {
                        worker.playback.state == PlaybackState::Playing
                            && (!s.pending.is_empty()
                                || (!s.eof
                                    && s.output.samples.len() < s.output.samples.capacity() / 2))
                    });
                    match receiver.recv_timeout(if hungry {
                        Duration::ZERO
                    } else {
                        Duration::from_millis(5)
                    }) {
                        Ok(command) => {
                            if let Err(error) = worker.command(command) {
                                worker.fail(error);
                            }
                            let _ = events.try_send(worker.playback.clone());
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                        Err(_) => {}
                    }
                    if let Err(error) = worker.pump() {
                        worker.fail(error);
                    }
                    if last.elapsed() >= Duration::from_millis(200) {
                        let _ = events.try_send(worker.playback.clone());
                        last = Instant::now();
                    }
                }
            })?;
        Ok(Self {
            commands,
            events: snapshots,
            shutdown,
        })
    }
    /// Never block the UI on a full queue or a busy decoder.
    pub fn send(&self, command: Command) -> Result<()> {
        self.commands
            .try_send(command)
            .map_err(|_| anyhow::anyhow!("Audio command queue unavailable"))
    }
}
impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

struct Session {
    reader: Reader,
    output: Output,
    converter: Converter,
    pending: VecDeque<f32>,
    eof: bool,
    base_ms: u64,
}
struct Worker {
    queue: Queue,
    playback: Playback,
    session: Option<Session>,
}

impl Worker {
    fn fail(&mut self, error: anyhow::Error) {
        self.session = None;
        self.playback.state = PlaybackState::Failed;
        self.playback.output_rate = None;
        self.playback.output_channels = None;
        self.playback.error = Some(format!("{error:#}"));
    }

    fn start_current(&mut self) -> Result<()> {
        self.session = None;
        self.playback.error = None;
        self.playback.position_ms = 0;
        let Some(track) = self.queue.current().cloned() else {
            self.playback.track = None;
            self.playback.duration_ms = 0;
            self.playback.output_rate = None;
            self.playback.output_channels = None;
            self.playback.state = PlaybackState::Stopped;
            return Ok(());
        };
        self.playback.track = Some(track.clone());
        anyhow::ensure!(track.available, "Track is unavailable");
        let reader = Reader::open(&track.path)?;
        let output = Output::open(reader.rate, reader.channels, self.playback.volume)?;
        let converter = Converter::new(
            reader.rate,
            reader.channels,
            output.rate,
            output.channels as usize,
        )?;
        self.playback.duration_ms = if reader.duration_ms > 0 {
            reader.duration_ms
        } else {
            track.metadata.duration_ms
        };
        self.playback.output_rate = Some(output.rate);
        self.playback.output_channels = Some(output.channels);
        self.playback.state = PlaybackState::Playing;
        self.session = Some(Session {
            reader,
            output,
            converter,
            pending: VecDeque::new(),
            eof: false,
            base_ms: 0,
        });
        Ok(())
    }

    fn command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Play { tracks, start } => {
                self.queue.set(tracks, start);
                self.start_current()?;
            }
            Command::Resume => {
                if self.session.is_some() {
                    self.playback.state = PlaybackState::Playing;
                } else {
                    self.start_current()?;
                }
            }
            Command::Pause => {
                if let Some(session) = &self.session {
                    session.output.playing.store(false, Ordering::Relaxed);
                    self.playback.state = PlaybackState::Paused;
                }
            }
            Command::Stop => {
                self.session = None;
                self.playback.state = PlaybackState::Stopped;
                self.playback.position_ms = 0;
                self.playback.error = None;
                self.playback.output_rate = None;
                self.playback.output_channels = None;
            }
            Command::Volume(value) => {
                if value.is_finite() {
                    self.playback.volume = value.clamp(0., 1.);
                }
                if let Some(session) = &self.session {
                    session
                        .output
                        .volume
                        .store(self.playback.volume.to_bits(), Ordering::Relaxed);
                }
            }
            Command::Shuffle(value) => {
                self.queue.shuffle(value);
                self.playback.shuffle = value;
            }
            Command::Repeat(value) => {
                self.queue.repeat = value;
                self.playback.repeat = value;
            }
            Command::Next => {
                if self.queue.next(false) {
                    self.start_current()?;
                } else {
                    self.session = None;
                    self.playback.state = PlaybackState::Stopped;
                    self.playback.position_ms = 0;
                }
            }
            Command::Previous => {
                if self.playback.position_ms > 3000 {
                    self.seek(0)?;
                } else if self.queue.previous() {
                    self.start_current()?;
                }
            }
            Command::Seek(position) => self.seek(position)?,
        }
        Ok(())
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        let Some(mut session) = self.session.take() else {
            return Ok(());
        };
        // Stop and drop the previous stream before publishing a new queue; samples
        // from before the seek can never leak into the replacement callback.
        session.output.playing.store(false, Ordering::Relaxed);
        let base_ms = session.reader.seek(position)?;
        let reader = session.reader;
        drop(session.output);
        let output = Output::open(reader.rate, reader.channels, self.playback.volume)?;
        let converter = Converter::new(
            reader.rate,
            reader.channels,
            output.rate,
            output.channels as usize,
        )?;
        self.session = Some(Session {
            reader,
            output,
            converter,
            pending: VecDeque::new(),
            eof: false,
            base_ms,
        });
        self.playback.position_ms = base_ms;
        Ok(())
    }

    fn pump(&mut self) -> Result<()> {
        let Some(session) = self.session.as_mut() else {
            return Ok(());
        };
        anyhow::ensure!(
            !session.output.failed.load(Ordering::Relaxed),
            "Audio output was disconnected or failed"
        );
        if self.playback.state != PlaybackState::Playing {
            return Ok(());
        }
        // At most one packet is decoded per turn, so command latency stays bounded
        // by one source read/decode, rather than a complete track.
        if session.pending.is_empty()
            && !session.eof
            && session.output.samples.len() < session.output.samples.capacity() / 2
        {
            match session.reader.next()? {
                Some(samples) => session
                    .pending
                    .extend(session.converter.process(&samples, false)?),
                None => {
                    session.eof = true;
                    session
                        .pending
                        .extend(session.converter.process(&[], true)?);
                }
            }
        }
        let channels = session.output.channels as usize;
        while session.pending.len() >= channels {
            let left = session.pending.pop_front().unwrap();
            let right = if channels == 2 {
                session.pending.pop_front().unwrap()
            } else {
                0.
            };
            if session.output.samples.push([left, right]).is_err() {
                if channels == 2 {
                    session.pending.push_front(right);
                }
                session.pending.push_front(left);
                break;
            }
        }
        // Prebuffer a short segment before enabling the callback, including after seek.
        if session.output.samples.len() >= session.output.rate as usize / 20 || session.eof {
            session.output.playing.store(true, Ordering::Relaxed);
        }
        self.playback.position_ms = session.base_ms + session.output.position_ms();
        if session.eof && session.pending.is_empty() && session.output.samples.is_empty() {
            self.session = None;
            if self.queue.next(true) {
                self.start_current()?;
            } else {
                self.playback.state = PlaybackState::Stopped;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/engine.rs"]
mod tests;
