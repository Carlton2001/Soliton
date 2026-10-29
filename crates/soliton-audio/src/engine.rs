use crate::{
    Command, Playback, PlaybackState, decoder::Reader, output::Output, queue::Queue,
    resample::Converter,
};
use anyhow::Result;
use soliton_core::failure::{Failure, Kind};
#[path = "fallback.rs"]
mod fallback;
#[path = "remote.rs"]
mod remote;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// Longest wait for a command while no local stream plays: device lists,
/// network states and shutdown are still noticed this fast.
const IDLE_WAIT: Duration = Duration::from_millis(100);

/// Completed callbacks after the last queued frame before a finished track's
/// stream may close: the device's own buffer still holds what they took.
const DRAIN_CALLBACKS: u64 = 3;
/// Bounds that wait when a device stops calling back.
const DRAIN_WAIT: Duration = Duration::from_millis(200);

/// A track failing after this long sits on a source that hangs rather than refuses.
const SLOW_FAILURE: Duration = Duration::from_secs(1);

pub struct Engine {
    commands: Sender<Command>,
    _outputs: crate::backend::Outputs,
    refresh_outputs: Sender<()>,
    /// Output changes are notified by the platform; polling is only a safety net.
    outputs_watched: Arc<AtomicBool>,
    pub events: Receiver<Playback>,
    shutdown: Arc<AtomicBool>,
    finished: Receiver<()>,
}

impl Engine {
    pub fn start(volume: f32, selected_output: Option<String>) -> Result<Self> {
        let (commands, receiver) = bounded(32);
        let (events, snapshots) = bounded(8);
        let (finished_tx, finished) = bounded(1);
        let (refresh_outputs, refresh) = bounded(1);
        let (discovered, devices) = bounded(1);
        let outputs_watched = Arc::new(AtomicBool::new(false));
        let outputs = crate::backend::watch_outputs(discovered, refresh, outputs_watched.clone())?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_shutdown = shutdown.clone();
        std::thread::Builder::new()
            .name("soliton-audio".into())
            .spawn(move || {
                let mut worker = Worker {
                    queue: Queue::default(),
                    playback: Playback {
                        volume: volume.clamp(0., 1.),
                        selected_output,
                        ..Default::default()
                    },
                    session: None,
                    network: None,
                    remote_generation: None,
                    resume_at: None,
                };
                let mut last = Instant::now() - Duration::from_secs(1);
                let mut deferred = None;
                while !worker_shutdown.load(Ordering::Relaxed) {
                    if let Ok(result) = devices.try_recv() {
                        match result {
                            Ok(outputs) => {
                                let previous = std::mem::replace(
                                    &mut worker.playback.outputs,
                                    Arc::new(outputs),
                                );
                                worker.playback.outputs_error = None;
                                worker.outputs_changed(&previous);
                            }
                            Err(error) => worker.playback.outputs_error = Some(Failure::of(&error)),
                        }
                    }
                    let hungry = worker.session.as_ref().is_some_and(|s| {
                        worker.playback.state == PlaybackState::Playing
                            && (!s.pending.is_empty()
                                || (!s.eof
                                    && s.output.samples.len() < s.output.samples.capacity() / 2))
                    });
                    let streaming =
                        worker.session.is_some() && worker.playback.state == PlaybackState::Playing;
                    let received = if let Some(command) = deferred.take() {
                        Ok(command)
                    } else {
                        receiver.recv_timeout(if hungry {
                            Duration::ZERO
                        } else if streaming {
                            Duration::from_millis(5)
                        } else {
                            IDLE_WAIT
                        })
                    };
                    match received {
                        Ok(mut command) => {
                            // A network round-trip or a held key must not turn
                            // repeated volume or seek commands into a backlog.
                            if matches!(command, Command::Volume(_) | Command::Seek(_)) {
                                let kind = std::mem::discriminant(&command);
                                while let Ok(next) = receiver.try_recv() {
                                    if std::mem::discriminant(&next) == kind {
                                        command = next;
                                    } else {
                                        deferred = Some(next);
                                        break;
                                    }
                                }
                            }
                            worker.playback.output_changing =
                                matches!(&command, Command::SelectNetworkOutput(_))
                                    || (worker.playback.network_output.is_some()
                                        && matches!(
                                            &command,
                                            Command::SelectOutput(_)
                                                | Command::Play { .. }
                                                | Command::PlayPlaylist { .. }
                                                | Command::Resume
                                        ));
                            if worker.playback.output_changing {
                                let _ = events.try_send(worker.snapshot());
                            }
                            if let Err(error) = worker.command(command)
                                && !worker.interrupt(&error)
                            {
                                worker.fail(error);
                            }
                            worker.playback.output_changing = false;
                            let _ = events.try_send(worker.snapshot());
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
                        Err(_) => {}
                    }
                    if let Err(error) = worker.pump() {
                        worker.recover(error);
                    }
                    if last.elapsed() >= Duration::from_millis(200) {
                        let _ = events.try_send(worker.snapshot());
                        last = Instant::now();
                    }
                }
                if let Some(network) = worker.network.take() {
                    network.shutdown();
                }
                let _ = finished_tx.try_send(());
            })?;
        Ok(Self {
            commands,
            _outputs: outputs,
            refresh_outputs,
            outputs_watched,
            events: snapshots,
            shutdown,
            finished,
        })
    }
    /// Set once the caller forwards native output changes as
    /// [`Command::RefreshOutputs`]; periodic enumeration then becomes rare.
    pub fn set_outputs_watched(&self, watched: bool) {
        self.outputs_watched.store(watched, Ordering::Relaxed);
    }

    /// Never block the UI on a full queue or a busy decoder.
    pub fn send(&self, command: Command) -> Result<()> {
        if matches!(command, Command::RefreshOutputs) {
            return match self.refresh_outputs.try_send(()) {
                Ok(()) | Err(crossbeam_channel::TrySendError::Full(_)) => Ok(()),
                Err(error) => Err(error.into()),
            };
        }
        self.commands
            .try_send(command)
            .map_err(|_| Failure::new(Kind::Busy).into())
    }

    /// Called on a background executor when closing the app, so the remote
    /// receiver and its authorized HTTP session can be shut down before exit.
    pub fn shutdown(self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = self.finished.recv_timeout(Duration::from_secs(20));
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
    /// Skipped passages are reported once per track.
    warned: bool,
    /// Output the stream was opened for; `None` follows the system default.
    target: Option<String>,
    /// Callback count and time when the last frame was queued at end of track.
    drain: Option<(u64, Instant)>,
}

impl Session {
    fn open(
        mut reader: Reader,
        volume: f32,
        selected: Option<&str>,
        position: Option<u64>,
    ) -> Result<Self> {
        let base_ms = match position {
            Some(position) => reader.seek(position)?,
            None => 0,
        };
        let output = Output::open(reader.rate, reader.channels, volume, selected)
            .map_err(|error| Failure::wrap(Kind::OutputUnavailable, &error))?;
        Self::with_output(reader, output, selected, base_ms)
    }

    /// Plays `reader`, already positioned at `base_ms`, on an open output.
    fn with_output(
        reader: Reader,
        output: Output,
        selected: Option<&str>,
        base_ms: u64,
    ) -> Result<Self> {
        let converter = Converter::new(
            reader.rate,
            reader.channels,
            output.rate,
            output.frame_channels,
        )?;
        Ok(Self {
            reader,
            output,
            converter,
            pending: VecDeque::new(),
            eof: false,
            warned: false,
            base_ms,
            target: selected.map(str::to_owned),
            drain: None,
        })
    }

    /// The stream of a finished track serves the next one when nothing about it
    /// would change: reopening a device is slow, notably over Bluetooth.
    fn reuse(self, reader: &Reader, target: Option<&str>) -> Option<Output> {
        let same = self.reader.rate == reader.rate
            && self.reader.channels == reader.channels
            && self.target.as_deref() == target
            && !self.output.failed.load(Ordering::Relaxed);
        same.then(|| {
            self.output.flush();
            self.output
        })
    }

    /// Whether the device played out everything queued for a finished track.
    fn drained(&mut self) -> bool {
        let callbacks = self.output.callbacks();
        let (start, since) = *self.drain.get_or_insert((callbacks, Instant::now()));
        callbacks.wrapping_sub(start) >= DRAIN_CALLBACKS || since.elapsed() >= DRAIN_WAIT
    }

    /// Decodes at most one packet, so command latency stays bounded by one
    /// source read/decode rather than a complete track, and moves pending
    /// frames to the device buffer. Returns whether more audio is wanted now.
    fn fill(&mut self) -> Result<bool> {
        let half = self.output.samples.capacity() / 2;
        if self.pending.is_empty() && !self.eof && self.output.samples.len() < half {
            match self.reader.next()? {
                Some(samples) => self
                    .pending
                    .extend(self.converter.process(&samples, false)?),
                None => {
                    self.eof = true;
                    self.pending.extend(self.converter.process(&[], true)?);
                }
            }
        }
        let channels = self.output.frame_channels;
        while self.pending.len() >= channels {
            let left = self.pending.pop_front().unwrap();
            let right = if channels == 2 {
                self.pending.pop_front().unwrap()
            } else {
                0.
            };
            if self.output.samples.push([left, right]).is_err() {
                if channels == 2 {
                    self.pending.push_front(right);
                }
                self.pending.push_front(left);
                break;
            }
        }
        // Prebuffer a short segment before enabling the callback, including after seek.
        if self.output.samples.len() >= self.output.rate as usize / 20 || self.eof {
            self.output.playing.store(true, Ordering::Relaxed);
        }
        Ok(!self.pending.is_empty() || (!self.eof && self.output.samples.len() < half))
    }

    /// Keeps a playing stream fed while the worker waits on something else;
    /// a failure stops feeding and is met again by the next regular turn.
    fn feeder(session: &mut Option<Self>, playing: bool) -> impl FnMut() -> bool + '_ {
        let mut failed = !playing;
        move || {
            let Some(session) = session.as_mut().filter(|_| !failed) else {
                return false;
            };
            session.fill().unwrap_or_else(|_| {
                failed = true;
                false
            })
        }
    }
}

#[derive(Default)]
struct Worker {
    queue: Queue,
    playback: Playback,
    session: Option<Session>,
    network: Option<soliton_network::Network>,
    remote_generation: Option<u64>,
    /// Position of a loaded queue, used when playback first starts.
    resume_at: Option<u64>,
}

impl Worker {
    fn snapshot(&self) -> Playback {
        let mut playback = Playback {
            queue: self.queue.snapshot.clone(),
            ..self.playback.clone()
        };
        if let Some(session) = &self.session
            && playback.output.is_some()
        {
            playback.output = Some(session.output.info());
        }
        playback
    }
    fn fail(&mut self, error: anyhow::Error) {
        if self.remote_generation.take().is_some()
            && let Some(network) = &self.network
        {
            let _ = network.call(soliton_network::Command::Stop);
        }
        self.session = None;
        self.playback.state = PlaybackState::Failed;
        self.playback.output = None;
        self.playback.error = Some(Failure::of(&error));
    }

    fn start_current(&mut self) -> Result<()> {
        self.start_current_at(0)
    }

    fn start_current_at(&mut self, position: u64) -> Result<()> {
        self.start_after(position, None)
    }

    /// Starts the current track, on the stream of `finished` when it fits.
    fn start_after(&mut self, position: u64, finished: Option<Session>) -> Result<()> {
        self.resume_at = None;
        self.session = None;
        self.playback.output = None;
        self.playback.error = None;
        self.playback.interruption = None;
        self.playback.position_ms = position;
        let Some(track) = self.queue.current().cloned() else {
            self.stop_remote()?;
            self.playback.track = None;
            self.playback.duration_ms = 0;
            self.playback.state = PlaybackState::Stopped;
            return Ok(());
        };
        self.playback.track = Some(track.clone());
        self.playback.duration_ms = track.metadata.duration_ms;
        if !track.available {
            let detail = track.path.display().to_string();
            return Err(Failure::with(Kind::TrackUnavailable, detail).into());
        }
        if self.playback.network_output.is_some() {
            return self.start_remote(track, position, true);
        }
        let reader = Reader::open(&track.path)?;
        self.playback.duration_ms = if reader.duration_ms > 0 {
            reader.duration_ms
        } else {
            track.metadata.duration_ms
        };
        let target = self.output_target().map(str::to_owned);
        let session = match finished.and_then(|s| s.reuse(&reader, target.as_deref())) {
            Some(output) => {
                let mut reader = reader;
                let base_ms = if position > 0 {
                    reader.seek(position)?
                } else {
                    0
                };
                Session::with_output(reader, output, target.as_deref(), base_ms)?
            }
            None => Session::open(
                reader,
                self.playback.volume,
                target.as_deref(),
                (position > 0).then_some(position),
            )?,
        };
        self.playback.output = Some(session.output.info());
        self.playback.state = PlaybackState::Playing;
        self.session = Some(session);
        Ok(())
    }

    fn command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::PlayPlaylist { entries, start } => {
                self.queue.set_entries(
                    entries
                        .into_iter()
                        .map(|(id, track)| (Some(id), track))
                        .collect(),
                    start,
                );
                self.start_current()?;
            }
            Command::LoadSelection {
                entries,
                start,
                position_ms,
            } => {
                self.stop_remote()?;
                self.session = None;
                self.queue.restore_entries(entries, start);
                let track = self.queue.current().cloned();
                self.playback.duration_ms = track.as_ref().map_or(0, |t| t.metadata.duration_ms);
                self.playback.position_ms = position_ms.min(self.playback.duration_ms);
                self.playback.state = if track.is_some() {
                    PlaybackState::Paused
                } else {
                    PlaybackState::Stopped
                };
                self.playback.track = track;
                self.playback.output = None;
                self.playback.error = None;
                self.playback.interruption = None;
                self.resume_at = Some(self.playback.position_ms);
            }
            Command::Enqueue { tracks, next } => {
                let empty = self.queue.current().is_none();
                self.queue.enqueue(tracks, next);
                if empty && let Some(track) = self.queue.current().cloned() {
                    self.playback.duration_ms = track.metadata.duration_ms;
                    self.playback.track = Some(track);
                    self.playback.position_ms = 0;
                    self.playback.state = PlaybackState::Paused;
                    self.resume_at = Some(0);
                }
            }
            Command::Availability(changes) => {
                self.queue.set_availability(&changes);
                if let Some(track) = &mut self.playback.track
                    && let Some(&(_, available)) = changes.iter().find(|(id, _)| *id == track.id)
                {
                    track.available = available;
                }
            }
            Command::RemoveQueued(id) => self.queue.remove(id),
            Command::MoveQueued { id, before } => self.queue.move_before(id, before),
            Command::ClearUpcoming => self.queue.clear_upcoming(),
            Command::PlayQueued(id) => {
                if self.queue.select(id) {
                    self.start_current()?;
                }
            }
            Command::Play { tracks, start } => {
                self.queue.set(tracks, start);
                self.start_current()?;
            }
            Command::Load {
                tracks,
                start,
                position_ms,
            } => {
                self.command(Command::LoadSelection {
                    entries: tracks.into_iter().map(|track| (None, track)).collect(),
                    start,
                    position_ms,
                })?;
            }
            Command::Resume => {
                if self.remote_generation.is_some() {
                    self.remote_command(soliton_network::Command::Play)?;
                } else if self.session.is_some() {
                    self.playback.state = PlaybackState::Playing;
                } else {
                    let position = self.resume_at.take().unwrap_or(0);
                    self.start_current_at(position)?;
                }
            }
            Command::Pause => {
                if self.remote_generation.is_some() {
                    self.remote_command(soliton_network::Command::Pause)?;
                } else if let Some(session) = &self.session {
                    session.output.playing.store(false, Ordering::Relaxed);
                    self.playback.state = PlaybackState::Paused;
                }
            }
            Command::Stop => {
                self.resume_at = None;
                self.stop_remote()?;
                self.session = None;
                self.playback.state = PlaybackState::Stopped;
                self.playback.position_ms = 0;
                self.playback.error = None;
                self.playback.interruption = None;
                self.playback.output = None;
            }
            Command::Volume(value) => {
                if self.remote_generation.is_some() && value.is_finite() {
                    if let Err(error) =
                        self.remote_command(soliton_network::Command::Volume(value.clamp(0., 1.)))
                    {
                        self.playback.output_error = Some(Failure::of(&error));
                    }
                    return Ok(());
                }
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
                    self.stop_remote()?;
                    self.session = None;
                    self.playback.output = None;
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
            Command::SelectOutput(selected) => {
                let result = self.select_output(selected);
                if result.is_ok() {
                    self.playback.fallback = false;
                }
                self.playback.output_error = result.err().map(|e| Failure::of(&e));
            }
            Command::RefreshOutputs => {}
            Command::DiscoverNetworkOutputs => {
                self.playback.network_error =
                    self.discover_network().err().map(|e| Failure::of(&e));
            }
            Command::SelectNetworkOutput(id) => {
                if let Err(error) = self.select_network(&id) {
                    if self.playback.network_output.is_some() {
                        self.fail(error);
                    } else {
                        self.playback.output_error = Some(Failure::of(&error));
                    }
                } else {
                    self.playback.output_error = None;
                }
            }
        }
        Ok(())
    }

    fn seek(&mut self, position: u64) -> Result<()> {
        if self.remote_generation.is_some() {
            return self.remote_command(soliton_network::Command::Seek(position));
        }
        let Some(session) = self.session.as_mut() else {
            // A loaded queue has no stream yet: it will start at this position.
            if self.resume_at.is_some() {
                let position = position.min(self.playback.duration_ms);
                self.resume_at = Some(position);
                self.playback.position_ms = position;
            }
            return Ok(());
        };
        // The device stream stays open: reopening it is slow, notably over
        // Bluetooth. Frames from before the seek are dropped, never played.
        session.output.flush();
        session.base_ms = session.reader.seek(position)?;
        session.converter = Converter::new(
            session.reader.rate,
            session.reader.channels,
            session.output.rate,
            session.output.frame_channels,
        )?;
        session.pending.clear();
        session.eof = false;
        self.playback.position_ms = session.base_ms;
        Ok(())
    }

    /// Prepare the replacement before dropping the old stream. A failed choice
    /// leaves both the working output and the persisted preference untouched.
    fn select_output(&mut self, selected: Option<String>) -> Result<()> {
        if self.playback.network_output.is_some() {
            return self.select_local(selected);
        }
        if self.session.is_some() {
            self.replace_output(selected.as_deref())?;
        } else if let Some(name) = &selected {
            crate::backend::check(name)?;
        }
        self.playback.selected_output = selected;
        Ok(())
    }

    /// Moves the active session to `target`, at the position played so far;
    /// a failure keeps the current stream.
    fn replace_output(&mut self, target: Option<&str>) -> Result<()> {
        let Some(current) = &self.session else {
            return Ok(());
        };
        let track = self
            .playback
            .track
            .as_ref()
            .expect("Active session has a track");
        let reader = Reader::open(&track.path)?;
        let mut session = Session::open(reader, self.playback.volume, target, None)?;
        // Slow device opening must not rewind the audio consumed meanwhile.
        // Pause only once the replacement is ready, then sample the position.
        let active = current.output.playing.swap(false, Ordering::Relaxed);
        let position = current.base_ms + current.output.position_ms();
        match session.reader.seek(position) {
            Ok(base_ms) => session.base_ms = base_ms,
            Err(error) => {
                current.output.playing.store(active, Ordering::Relaxed);
                return Err(error);
            }
        }
        self.playback.position_ms = session.base_ms;
        self.playback.output = Some(session.output.info());
        self.session = Some(session);
        Ok(())
    }

    /// A track that cannot be read is skipped during automatic playback, as long
    /// as another one can start. Local output failures pause playback; network
    /// failures stop it. A slow failure means a hung source: the next tracks
    /// would each block the worker as long, so skipping stops there.
    fn recover(&mut self, mut error: anyhow::Error) {
        for _ in 0..self.queue.snapshot.entries.len() {
            let failure = Failure::of(&error);
            let track_failed = matches!(
                failure.kind,
                Kind::TrackUnavailable | Kind::UnreadableAudio | Kind::UnsupportedAudio
            );
            if !track_failed || self.remote_generation.is_some() {
                break;
            }
            self.playback.warning = Some(failure);
            self.session = None;
            self.playback.output = None;
            // Repeating a track that cannot be read would never end.
            if !self.queue.next(false) {
                self.playback.state = PlaybackState::Stopped;
                self.playback.position_ms = 0;
                return;
            }
            let started = Instant::now();
            match self.start_current() {
                Ok(()) => return,
                Err(next) => error = next,
            }
            if started.elapsed() >= SLOW_FAILURE {
                break;
            }
        }
        if !self.interrupt(&error) {
            self.fail(error);
        }
    }

    fn pump(&mut self) -> Result<()> {
        self.poll_network()?;
        if self.remote_generation.is_some() {
            return Ok(());
        }
        if let Some(session) = &self.session
            && session.output.failed.load(Ordering::Relaxed)
            && !self.follow_default()
        {
            let label = self
                .session
                .as_ref()
                .map(|session| session.output.info().device_label)
                .unwrap_or_default();
            return Err(if label.is_empty() {
                Failure::new(Kind::OutputDisconnected)
            } else {
                Failure::with(Kind::OutputDisconnected, label)
            }
            .into());
        }
        let Some(session) = self.session.as_mut() else {
            return Ok(());
        };
        if self.playback.state != PlaybackState::Playing {
            return Ok(());
        }
        session.fill()?;
        if session.reader.skipped > 0 && !session.warned {
            session.warned = true;
            let mut warning = Failure::new(Kind::SkippedAudio);
            if let Some(track) = &self.playback.track {
                warning = warning.about(track.path.display());
            }
            self.playback.warning = Some(warning);
        }
        self.playback.position_ms = session.base_ms + session.output.position_ms();
        if session.eof
            && session.pending.is_empty()
            && session.output.samples.is_empty()
            && session.drained()
        {
            let finished = self.session.take();
            self.playback.output = None;
            if self.queue.next(true) {
                self.start_after(0, finished)?;
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
