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
    /// Library tracks of the queue, in the order sent to the engine.
    queue: Vec<i64>,
    /// The saved queue still has to be loaded once the library is read.
    pending: bool,
    /// Track being loaded: engine updates older than the load are not saved.
    loading: Option<i64>,
}

impl Resume {
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            pending: true,
            loading: None,
        }
    }

    /// Notes the queue of a new playback.
    pub fn played(&mut self, command: &Command) {
        if let Command::Play { tracks, .. } = command {
            self.queue = tracks.iter().map(|t| t.id).filter(|&id| id > 0).collect();
            self.pending = false;
            self.loading = None;
        }
    }
}

impl Rap {
    fn current_queue(&self) -> LastQueue {
        let current = self
            .playback
            .track
            .as_ref()
            .map(|t| t.id)
            .filter(|id| self.resume.queue.contains(id));
        LastQueue {
            position_ms: if current.is_some() {
                self.playback.position_ms
            } else {
                0
            },
            tracks: if current.is_some() {
                self.resume.queue.clone()
            } else {
                Vec::new()
            },
            current,
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
            || queue.tracks != saved.tracks
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
        let tracks: Vec<_> = saved
            .tracks
            .iter()
            .filter_map(|id| known.get(id).map(|track| (*track).clone()))
            .collect();
        let Some(start) = tracks.iter().position(|t| t.id == current) else {
            return;
        };
        self.resume.queue = tracks.iter().map(|t| t.id).collect();
        self.resume.loading = Some(current);
        self.audio_command(
            Command::Load {
                tracks,
                start,
                position_ms: saved.position_ms,
            },
            cx,
        );
    }
}
