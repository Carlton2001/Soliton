//! Lost or missing local outputs. Following the system default, playback goes
//! on with the next default. A chosen output is never replaced by itself:
//! playback pauses where it was, resuming plays on the system default until
//! the chosen output is listed again, then playback returns to it.
use super::*;

impl Worker {
    /// The output streams open on: the chosen one, unless it is missing.
    pub(super) fn output_target(&self) -> Option<&str> {
        if self.playback.fallback {
            None
        } else {
            self.playback.selected_output.as_deref()
        }
    }

    /// With the system default chosen, a lost output (a headset turned off) is
    /// replaced by the next default without pausing. `false` when none opens,
    /// or when the lost stream never ran, so a failing default cannot loop.
    pub(super) fn follow_default(&mut self) -> bool {
        let ran = self
            .session
            .as_ref()
            .is_some_and(|session| session.output.ran());
        ran && self.playback.selected_output.is_none()
            && self.playback.network_output.is_none()
            && self.remote_generation.is_none()
            && self.replace_output(None).is_ok()
    }

    /// Pauses at the current position on local output failures; `false`, and
    /// nothing changed, for any other error.
    pub(super) fn interrupt(&mut self, error: &anyhow::Error) -> bool {
        let failure = Failure::of(error);
        let output_failed = matches!(
            failure.kind,
            Kind::OutputDisconnected
                | Kind::OutputUnavailable
                | Kind::OutputAmbiguous
                | Kind::NoOutputDevice
        );
        if !output_failed
            || self.remote_generation.is_some()
            || self.playback.network_output.is_some()
            || self.playback.track.is_none()
        {
            return false;
        }
        if self.playback.selected_output.is_some() {
            self.playback.fallback = true;
        }
        self.session = None;
        self.playback.output = None;
        self.playback.state = PlaybackState::Paused;
        self.resume_at = Some(self.playback.position_ms);
        self.playback.interruption = Some(failure);
        true
    }

    /// Returns to the chosen output once it reappears in the listing, keeping
    /// the position; a failed return stays on the system default. Only a
    /// transition counts: a listing sent before the loss was noticed may still
    /// contain the lost output.
    pub(super) fn outputs_changed(&mut self, previous: &[crate::OutputDevice]) {
        if !self.playback.fallback {
            return;
        }
        let Some(name) = self.playback.selected_output.clone() else {
            self.playback.fallback = false;
            return;
        };
        let listed = |outputs: &[crate::OutputDevice]| {
            outputs
                .iter()
                .any(|device| device.name == name && device.selectable)
        };
        if listed(previous) || !listed(&self.playback.outputs) {
            return;
        }
        self.playback.fallback = false;
        if self.session.is_some()
            && self.playback.network_output.is_none()
            && let Err(error) = self.select_output(Some(name))
        {
            self.playback.fallback = true;
            self.playback.output_error = Some(Failure::of(&error));
        }
    }
}
