//! Network routing on the audio command worker. A single queue remains the source
//! of truth for albums, shuffle, repeat and manual/automatic navigation.
use super::*;
use rap_core::model::Track;
use rap_network::{Command as RemoteCommand, State as RemoteState, Status};

impl Worker {
    pub(super) fn discover_network(&mut self) -> Result<()> {
        if self.network.is_none() {
            self.network = Some(rap_network::Network::start()?);
        }
        self.playback.network_enabled = true;
        self.playback.network_discovering = true;
        self.network.as_ref().unwrap().refresh()
    }

    pub(super) fn select_network(&mut self, id: &str) -> Result<()> {
        let device = self
            .playback
            .network_outputs
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or(Failure::new(Kind::NetworkUnavailable))?;
        let network = self
            .network
            .as_ref()
            .ok_or(Failure::new(Kind::NetworkUnavailable))?;
        network.call(RemoteCommand::Connect(device.clone()))?;
        let active = matches!(
            self.playback.state,
            PlaybackState::Playing | PlaybackState::Paused
        );
        let status = if active && let Some(track) = self.queue.current() {
            let position = self
                .session
                .as_ref()
                .map_or(self.playback.position_ms, |s| {
                    s.base_ms + s.output.position_ms()
                });
            network.call(RemoteCommand::Load {
                track: track.clone(),
                position_ms: position,
                autoplay: self.playback.state == PlaybackState::Playing,
                volume: self.playback.volume,
            })?
        } else {
            None
        };
        // The old local stream remains usable until the receiver accepts the media.
        self.session = None;
        self.playback.output = None;
        self.playback.network_output = Some(device);
        self.remote_generation = status.as_ref().map(|s| s.generation);
        if let Some(status) = status {
            self.apply_remote(&status)?;
        }
        Ok(())
    }

    pub(super) fn start_remote(
        &mut self,
        track: Track,
        position_ms: u64,
        autoplay: bool,
    ) -> Result<()> {
        let device = self
            .playback
            .network_output
            .clone()
            .ok_or(Failure::new(Kind::NetworkUnavailable))?;
        let network = self
            .network
            .as_ref()
            .ok_or(Failure::new(Kind::NetworkUnavailable))?;
        network.call(RemoteCommand::Connect(device))?;
        let status = network
            .call(RemoteCommand::Load {
                track,
                position_ms,
                autoplay,
                volume: self.playback.volume,
            })?
            .ok_or(Failure::new(Kind::NetworkPlaybackFailed))?;
        self.remote_generation = Some(status.generation);
        self.playback.error = None;
        self.apply_remote(&status)
    }

    pub(super) fn remote_command(&mut self, command: RemoteCommand) -> Result<()> {
        let status = self
            .network
            .as_ref()
            .ok_or(Failure::new(Kind::NetworkUnavailable))?
            .call(command)?
            .ok_or(Failure::new(Kind::NetworkPlaybackFailed))?;
        self.apply_remote(&status)
    }

    pub(super) fn stop_remote(&mut self) -> Result<()> {
        if self.remote_generation.take().is_some() {
            self.network
                .as_ref()
                .ok_or(Failure::new(Kind::NetworkUnavailable))?
                .call(RemoteCommand::Stop)?;
        }
        Ok(())
    }

    pub(super) fn select_local(&mut self, selected: Option<String>) -> Result<()> {
        let active = self.remote_generation.is_some()
            && matches!(
                self.playback.state,
                PlaybackState::Playing | PlaybackState::Paused
            );
        let replacement = if active && let Some(track) = self.queue.current() {
            let reader = Reader::open(&track.path)?;
            Some(Session::open(
                reader,
                self.playback.volume,
                selected.as_deref(),
                Some(self.playback.position_ms),
            )?)
        } else {
            if selected.is_some() {
                crate::devices::resolve(selected.as_deref())?;
            }
            None
        };
        self.network
            .as_ref()
            .ok_or(Failure::new(Kind::NetworkUnavailable))?
            .call(RemoteCommand::Disconnect)?;
        self.remote_generation = None;
        self.playback.network_output = None;
        self.playback.selected_output = selected;
        self.playback.output = replacement.as_ref().map(|s| s.output.info.clone());
        if let Some(session) = &replacement {
            self.playback.position_ms = session.base_ms;
        }
        self.session = replacement;
        Ok(())
    }

    fn apply_remote(&mut self, status: &Status) -> Result<()> {
        if status.state == RemoteState::Failed {
            let detail = status.error.clone().unwrap_or_default();
            return Err(Failure::with(Kind::NetworkPlaybackFailed, detail).into());
        }
        self.playback.position_ms = status.position_ms;
        if status.duration_ms > 0 {
            self.playback.duration_ms = status.duration_ms;
        }
        if let Some(volume) = status.volume.filter(|v| v.is_finite()) {
            self.playback.volume = volume.clamp(0., 1.);
        }
        self.playback.state = match status.state {
            RemoteState::Playing | RemoteState::Buffering => PlaybackState::Playing,
            RemoteState::Paused => PlaybackState::Paused,
            RemoteState::Failed => PlaybackState::Failed,
            _ => PlaybackState::Stopped,
        };
        Ok(())
    }

    pub(super) fn poll_network(&mut self) -> Result<()> {
        let Some(snapshot) = self
            .network
            .as_mut()
            .and_then(rap_network::Network::take_update)
        else {
            return Ok(());
        };
        self.playback.network_outputs = snapshot.devices;
        self.playback.network_error = snapshot
            .discovery_error
            .map(|error| Failure::with(Kind::Other, error));
        self.playback.network_discovering = snapshot.discovering;
        let Some(status) = snapshot
            .status
            .filter(|s| Some(s.generation) == self.remote_generation)
        else {
            return Ok(());
        };
        self.apply_remote(&status)?;
        if status.state == RemoteState::Finished {
            self.remote_generation = None;
            if self.queue.next(true) {
                self.start_current()?;
            }
        } else if status.state == RemoteState::Stopped {
            self.remote_generation = None;
        }
        Ok(())
    }
}
