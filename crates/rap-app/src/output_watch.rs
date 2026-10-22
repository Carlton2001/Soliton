//! Native audio output notifications: the engine re-enumerates its outputs
//! after a change instead of polling them every few seconds.
use crate::app::Rap;
use gpui::*;
use rap_audio::Command;
use rap_platform::{AudioOutputs, DeviceWatch};
use std::time::{Duration, Instant};

/// Pause after the last change: plugging a device raises several events.
const SETTLE: Duration = Duration::from_millis(300);

#[derive(Default)]
pub struct OutputWatch {
    watch: Option<DeviceWatch>,
    starting: bool,
    failed: bool,
    /// Last change not yet followed by an enumeration.
    changed: Option<Instant>,
}

impl Rap {
    pub fn poll_outputs(&mut self, cx: &mut Context<Self>) {
        if self.audio.is_none() || self.quitting {
            return;
        }
        let state = &mut self.output_watch;
        if state.watch.is_none() && !state.starting && !state.failed {
            state.starting = true;
            let start = cx
                .background_executor()
                .spawn(async { AudioOutputs::watch() });
            cx.spawn(async move |this, cx| {
                let result = start.await;
                let _ = this.update(cx, |this, _| {
                    let state = &mut this.output_watch;
                    state.starting = false;
                    match result {
                        Ok(watch) => {
                            state.watch = Some(watch);
                            if let Some(audio) = &this.audio {
                                audio.set_outputs_watched(true);
                            }
                        }
                        // The engine keeps polling its outputs on its own.
                        Err(error) => {
                            tracing::warn!("Audio output notifications unavailable: {error:#}");
                            state.failed = true;
                        }
                    }
                });
            })
            .detach();
            return;
        }
        if let Some(watch) = &state.watch
            && watch.changes.try_recv().is_ok()
        {
            state.changed = Some(Instant::now());
        }
        if state.changed.is_some_and(|last| last.elapsed() >= SETTLE) {
            state.changed = None;
            if let Some(audio) = &self.audio {
                let _ = audio.send(Command::RefreshOutputs);
            }
        }
    }
}
