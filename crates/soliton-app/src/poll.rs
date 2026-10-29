//! Applies what background work reports: engine states, service events and
//! catalog query results, on each tick of the interface loop.
use super::*;

impl Soliton {
    /// Applies engine states, service events and query results.
    pub(super) fn poll(&mut self, last_refresh: &mut Instant, cx: &mut Context<Self>) {
        self.poll_bluetooth(cx);
        self.poll_outputs(cx);
        let mut changed = false;
        if let Some(playback) = self.audio.as_ref().and_then(|a| a.events.try_iter().last()) {
            if playback.queue.current != self.playback.queue.current
                && let Some(index) = playback
                    .queue
                    .entries
                    .iter()
                    .position(|e| Some(e.id) == playback.queue.current)
            {
                self.queue_scroll
                    .scroll_to_item(index, ScrollStrategy::Center);
            }
            changed |= playback.state != self.playback.state
                || !Arc::ptr_eq(&playback.queue, &self.playback.queue)
                || playback.position_ms != self.playback.position_ms
                || playback.volume != self.playback.volume
                || playback.shuffle != self.playback.shuffle
                || playback.repeat != self.playback.repeat
                || playback.error != self.playback.error
                || playback.output != self.playback.output
                || playback.selected_output != self.playback.selected_output
                || playback.outputs != self.playback.outputs
                || playback.output_error != self.playback.output_error
                || playback.fallback != self.playback.fallback
                || playback.interruption != self.playback.interruption
                || playback.output_changing != self.playback.output_changing
                || playback.outputs_error != self.playback.outputs_error
                || playback.network_outputs != self.playback.network_outputs
                || playback.network_output != self.playback.network_output
                || playback.network_discovering != self.playback.network_discovering
                || playback.network_enabled != self.playback.network_enabled
                || playback.network_error != self.playback.network_error
                || playback.warning != self.playback.warning
                || playback.track.as_ref().map(|t| t.id)
                    != self.playback.track.as_ref().map(|t| t.id);
            if let Some(error) = &playback.error
                && playback.error != self.playback.error
            {
                self.report_failure(Level::Error, Text::Playback, error);
            }
            if let Some(warning) = &playback.warning
                && playback.warning != self.playback.warning
            {
                self.report_failure(Level::Warning, Text::Playback, warning);
            }
            let output_errors: Vec<_> = [
                (
                    &playback.output_error,
                    &self.playback.output_error,
                    Text::AudioOutput,
                ),
                (
                    &playback.outputs_error,
                    &self.playback.outputs_error,
                    Text::AudioOutput,
                ),
                (
                    &playback.network_error,
                    &self.playback.network_error,
                    Text::NetworkOutput,
                ),
            ]
            .into_iter()
            .filter_map(|(error, previous, label)| {
                error
                    .as_ref()
                    .filter(|_| error != previous)
                    .map(|error| (label, error.clone()))
            })
            .collect();
            for (label, error) in output_errors {
                self.report_failure(Level::Error, label, &error);
            }
            // Save only an acknowledged choice; rejected changes keep the old one.
            if playback.selected_output != self.settings.output_device {
                self.settings.output_device = playback.selected_output.clone();
                self.save();
            }
            if playback.network_output.is_some() {
                self.settings.volume = playback.volume;
            }
            let previous_track = self.playback.track.as_ref().map(|t| t.id);
            self.playback = playback;
            self.settle_seek(previous_track);
            self.remember_queue();
        }
        for event in self.services.events.try_iter().collect::<Vec<_>>() {
            match event {
                Event::Playlists(lists) => {
                    self.playlists.lists = lists;
                    self.playlists.pending = false;
                    if self
                        .playlists
                        .selected
                        .is_some_and(|id| !self.playlists.lists.iter().any(|p| p.id == id))
                    {
                        self.playlists.selected = None;
                    }
                }
                Event::Progress(progress) => self.progress = progress,
                Event::Finished(cancelled) => {
                    self.scanning = false;
                    if cancelled {
                        self.report_warning(self.t(Text::Cancelled));
                    }
                    self.query_pending = true;
                }
                Event::Removed | Event::Changed => self.query_pending = true,
                Event::SourceUnavailable(path) => {
                    let detail = if self.settings.hide_unavailable {
                        Text::SourceUnavailableHidden
                    } else {
                        Text::SourceUnavailable
                    };
                    let text = self.labelled(&path.display().to_string(), self.t(detail));
                    self.report_warning(text);
                    self.query_pending = true;
                }
                Event::Warning(failure) => {
                    self.report_failure(Level::Warning, service_label(&failure), &failure)
                }
                Event::Error(failure) => {
                    self.report_failure(Level::Error, service_label(&failure), &failure)
                }
            }
            changed = true;
        }
        for result in self.services.views.try_iter().collect::<Vec<_>>() {
            match result {
                Ok(view) if view.filter == self.filter => {
                    self.tracks = view.tracks;
                    self.snapshot = view.snapshot;
                    self.restore_queue(cx);
                    // A restored or open album may have left the library.
                    if self
                        .selected
                        .is_some_and(|id| !self.snapshot.albums.iter().any(|a| a.id == id))
                    {
                        self.selected = None;
                    }
                    self.visible = view.visible;
                    self.facets = view.facets;
                    self.filter_options = view.options;
                    self.keyboard_index = self
                        .keyboard_index
                        .min(self.visible.len().saturating_sub(1));
                }
                Ok(_) => {}
                Err(error) => {
                    self.report_failure(Level::Error, Text::Library, &Failure::of(&error))
                }
            }
            changed = true;
        }
        if self.scanning && last_refresh.elapsed() > Duration::from_millis(750) {
            self.query_pending = true;
            *last_refresh = Instant::now();
        }
        self.sync_queue_availability();
        if self.query_pending && self.services.query.try_send(self.filter.clone()).is_ok() {
            self.query_pending = false;
        }
        if changed {
            cx.notify();
        }
    }
}

/// Library workers also save settings; their failures say which.
fn service_label(failure: &Failure) -> Text {
    match failure.kind {
        Kind::SettingsNotSaved => Text::Settings,
        _ => Text::Library,
    }
}
