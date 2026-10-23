//! Keeps the play queue across sessions: library tracks, the current one and
//! its position. RAP reopens paused there; files played without being added
//! to the library are not kept.
use crate::app::Rap;
use gpui::*;
use rap_audio::{Command, PlaybackState};
use rap_core::config::LastQueue;
use std::collections::HashMap;

/// Position changes saved during playback, at most this often.
const SAVE_STEP_MS: u64 = 10_000;

#[derive(Default)]
pub struct Resume {
    /// The saved queue still has to be loaded once the library is read.
    pending: bool,
    /// Track being loaded: engine updates older than the load are not saved.
    loading: Option<i64>,
}

impl Resume {
    pub fn new() -> Self {
        Self {
            pending: true,
            loading: None,
        }
    }
}

impl Rap {
    fn current_queue(&self) -> LastQueue {
        let entries: Vec<_> = self
            .playback
            .queue
            .entries
            .iter()
            .filter(|e| e.track.id > 0)
            .collect();
        let current_index = entries
            .iter()
            .position(|e| Some(e.id) == self.playback.queue.current);
        let current = current_index.map(|i| entries[i].track.id);
        LastQueue {
            playlist_entries: entries.iter().map(|e| e.playlist_entry).collect(),
            position_ms: if current.is_some() {
                self.playback.position_ms
            } else {
                0
            },
            tracks: entries.iter().map(|e| e.track.id).collect(),
            current,
            current_index,
        }
    }

    /// Saves the queue when the track changes, when playback stops or pauses,
    /// and every few seconds of playback.
    pub fn remember_queue(&mut self) {
        if self.resume.pending {
            return;
        }
        if let Some(id) = self.resume.loading {
            if self.playback.track.as_ref().map(|t| t.id) != Some(id) {
                return;
            }
            self.resume.loading = None;
        }
        let queue = self.current_queue();
        let saved = &self.settings.last_queue;
        let moved = queue.position_ms.abs_diff(saved.position_ms);
        let worth = queue.current != saved.current
            || queue.current_index != saved.current_index
            || queue.tracks != saved.tracks
            || queue.playlist_entries != saved.playlist_entries
            || (moved > 0 && self.playback.state != PlaybackState::Playing)
            || moved >= SAVE_STEP_MS;
        if worth {
            self.settings.last_queue = queue;
            self.save();
        }
    }

    /// Latest state, written by the shutdown save.
    pub fn final_queue(&mut self) {
        if !self.resume.pending && self.resume.loading.is_none() {
            self.settings.last_queue = self.current_queue();
        }
    }

    /// Loads the saved queue paused, once the library snapshot is available.
    pub fn restore_queue(&mut self, cx: &mut Context<Self>) {
        // An empty snapshot may precede the library: wait for its content.
        if !self.resume.pending || self.audio.is_none() || self.snapshot.albums.is_empty() {
            return;
        }
        self.resume.pending = false;
        let saved = self.settings.last_queue.clone();
        let Some(current) = saved.current else {
            return;
        };
        let known: HashMap<i64, &rap_core::model::Track> = self
            .snapshot
            .albums
            .iter()
            .flat_map(|album| album.tracks())
            .map(|track| (track.id, track))
            .collect();
        let entries: Vec<_> = saved
            .tracks
            .iter()
            .enumerate()
            .filter_map(|(index, id)| {
                known.get(id).map(|track| {
                    (
                        saved.playlist_entries.get(index).copied().flatten(),
                        (*track).clone(),
                    )
                })
            })
            .collect();
        let start = saved
            .current_index
            .filter(|&i| saved.tracks.get(i) == Some(&current))
            .and_then(|i| {
                let known_before = saved.tracks[..i]
                    .iter()
                    .filter(|id| known.contains_key(id))
                    .count();
                entries
                    .get(known_before)
                    .filter(|(_, t)| t.id == current)
                    .map(|_| known_before)
            })
            .or_else(|| entries.iter().position(|(_, t)| t.id == current));
        let Some(start) = start else {
            return;
        };
        self.resume.loading = Some(current);
        self.audio_command(
            Command::LoadSelection {
                entries,
                start,
                position_ms: saved.position_ms,
            },
            cx,
        );
    }
}
