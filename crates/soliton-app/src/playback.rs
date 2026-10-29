//! Playback commands from the interface. The engine runs on its own threads.
use crate::{app::Soliton, i18n::Text, notices::Level};
use gpui::*;
use soliton_audio::{Command, PlaybackState, QueueSnapshot};
use soliton_core::{
    failure::Failure,
    model::{Album, Track},
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

/// A reported position this close to a sent seek shows that the engine applied it.
const SEEK_SETTLED_MS: u64 = 1_000;
/// A seek the engine never confirms, such as one past the end, stops being shown.
const SEEK_TIMEOUT: Duration = Duration::from_secs(2);

/// Whether a sent seek no longer needs to stand in for the reported position.
fn seek_settled(position_ms: u64, target_ms: u64, elapsed: Duration, track_changed: bool) -> bool {
    track_changed || elapsed >= SEEK_TIMEOUT || position_ms.abs_diff(target_ms) < SEEK_SETTLED_MS
}

/// Library tracks whose queued copy disagrees with the catalog; a track gone
/// from the library is unavailable.
fn stale_availability(queue: &QueueSnapshot, tracks: &HashMap<i64, Track>) -> Vec<(i64, bool)> {
    queue
        .entries
        .iter()
        .filter(|e| e.track.id > 0)
        .filter_map(|e| {
            let available = tracks.get(&e.track.id).is_some_and(|t| t.available);
            (available != e.track.available).then_some((e.track.id, available))
        })
        .collect()
}

impl Soliton {
    pub fn audio_command(&mut self, command: Command, cx: &mut Context<Self>) {
        if self.playback.output_changing
            && matches!(
                &command,
                Command::SelectOutput(_) | Command::SelectNetworkOutput(_)
            )
        {
            return;
        }
        if let Some(audio) = &self.audio
            && let Err(error) = audio.send(command)
        {
            self.report_failure(Level::Error, Text::Playback, &Failure::of(&error));
        }
        cx.notify();
    }

    /// Keeps the engine's queued copies as available as the catalog shows them,
    /// so that a source back online plays again. Checked once per queue or
    /// catalog change; a command refused by a busy engine is retried.
    pub fn sync_queue_availability(&mut self) {
        let Some(audio) = &self.audio else {
            return;
        };
        if self
            .availability_checked
            .as_ref()
            .is_some_and(|(queue, tracks)| {
                Arc::ptr_eq(queue, &self.playback.queue) && Arc::ptr_eq(tracks, &self.tracks)
            })
        {
            return;
        }
        let changes = stale_availability(&self.playback.queue, &self.tracks);
        if changes.is_empty() || audio.send(Command::Availability(changes)).is_ok() {
            self.availability_checked = Some((self.playback.queue.clone(), self.tracks.clone()));
        }
    }

    /// Plays one edition of an album from `start`, skipping unavailable tracks.
    pub fn play_album(&mut self, id: i64, edition: usize, start: usize, cx: &mut Context<Self>) {
        let Some(edition) = self
            .snapshot
            .albums
            .iter()
            .find(|a| a.id == id)
            .and_then(|album| album.editions.get(edition))
        else {
            return;
        };
        if edition.tracks.get(start).is_none_or(|t| !t.available) {
            return;
        }
        let start_id = edition.tracks[start].id;
        let tracks: Vec<_> = edition
            .tracks
            .iter()
            .filter(|t| t.available)
            .cloned()
            .collect();
        let start = tracks.iter().position(|t| t.id == start_id).unwrap_or(0);
        self.audio_command(Command::Play { tracks, start }, cx);
    }

    /// Plays the displayed albums in their current order, each in its best edition.
    pub fn play_all(&mut self, cx: &mut Context<Self>) {
        let tracks: Vec<_> = self
            .visible
            .iter()
            .flat_map(|&index| self.snapshot.albums[index].primary().tracks.iter())
            .filter(|t| t.available)
            .cloned()
            .collect();
        if !tracks.is_empty() {
            self.audio_command(Command::Play { tracks, start: 0 }, cx);
        }
    }

    pub fn enqueue_album(&mut self, id: i64, edition: usize, next: bool, cx: &mut Context<Self>) {
        if let Some(edition) = self
            .snapshot
            .albums
            .iter()
            .find(|a| a.id == id)
            .and_then(|a| a.editions.get(edition))
        {
            let tracks = edition
                .tracks
                .iter()
                .filter(|t| t.available)
                .cloned()
                .collect::<Vec<_>>();
            if !tracks.is_empty() {
                self.audio_command(Command::Enqueue { tracks, next }, cx);
            }
        }
    }

    pub fn play_pause(&mut self, cx: &mut Context<Self>) {
        if self.playback.state == PlaybackState::Playing {
            self.audio_command(Command::Pause, cx);
        } else if self.playback.track.is_some() {
            self.audio_command(Command::Resume, cx);
        } else if let Some(id) = self
            .selected
            .or_else(|| self.visible.first().map(|&i| self.snapshot.albums[i].id))
        {
            let edition = if self.selected == Some(id) {
                self.selected_edition
            } else {
                0
            };
            let start = self
                .snapshot
                .albums
                .iter()
                .find(|a| a.id == id)
                .and_then(|a| a.editions.get(edition))
                .and_then(|e| e.tracks.iter().position(|t| t.available));
            if let Some(start) = start {
                self.play_album(id, edition, start, cx);
            }
        }
    }

    /// `save` persists the value; dragging only sends it to the engine.
    pub fn set_volume(&mut self, value: f32, save: bool, cx: &mut Context<Self>) {
        self.settings.volume = value.clamp(0., 1.);
        self.muted_volume = None;
        if save {
            self.save();
        }
        self.audio_command(Command::Volume(self.settings.volume), cx);
    }

    /// Kept for the next sessions, like the volume.
    pub fn set_shuffle(&mut self, value: bool, cx: &mut Context<Self>) {
        self.settings.shuffle = value;
        self.save();
        self.audio_command(Command::Shuffle(value), cx);
    }

    pub fn set_repeat(&mut self, value: soliton_audio::Repeat, cx: &mut Context<Self>) {
        self.settings.repeat = value;
        self.save();
        self.audio_command(Command::Repeat(value), cx);
    }

    pub fn toggle_mute(&mut self, cx: &mut Context<Self>) {
        let restored = self.muted_volume.take();
        match restored {
            Some(volume) => self.set_volume(volume, true, cx),
            None if self.settings.volume > 0. => {
                let volume = self.settings.volume;
                self.set_volume(0., true, cx);
                self.muted_volume = Some(volume);
            }
            None => self.set_volume(0.5, true, cx),
        }
    }

    /// Position slider: previews while dragging, seeks once released.
    pub fn seek_fraction(&mut self, fraction: f32, released: bool, cx: &mut Context<Self>) {
        if released {
            self.seek_preview = None;
            let target = (self.playback.duration_ms as f64 * f64::from(fraction)) as u64;
            self.seek_target = Some((target, Instant::now()));
            self.audio_command(Command::Seek(target), cx);
        } else {
            self.seek_preview = Some(fraction);
            cx.notify();
        }
    }

    /// Position to display: a sent seek, until the engine reports it, rather
    /// than the earlier position still in flight.
    pub fn shown_position(&self) -> u64 {
        self.seek_target
            .filter(|(_, sent)| sent.elapsed() < SEEK_TIMEOUT)
            .map_or(self.playback.position_ms, |(target, _)| target)
    }

    /// Called with each engine state, after it replaced `previous_track`'s.
    pub fn settle_seek(&mut self, previous_track: Option<i64>) {
        let track = self.playback.track.as_ref().map(|t| t.id);
        if let Some((target, sent)) = self.seek_target
            && seek_settled(
                self.playback.position_ms,
                target,
                sent.elapsed(),
                track != previous_track,
            )
        {
            self.seek_target = None;
        }
    }

    /// Album of the current track, when it comes from the library.
    pub fn current_album(&self) -> Option<&Album> {
        let id = self.playback.track.as_ref()?.id;
        self.snapshot
            .albums
            .iter()
            .find(|album| album.tracks().any(|t| t.id == id))
    }
}

#[cfg(test)]
#[path = "tests/playback.rs"]
mod tests;
