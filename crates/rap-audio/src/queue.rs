use crate::{QueueEntry, QueueSnapshot};
use rap_core::model::Track;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Repeat {
    #[default]
    Off,
    Track,
    All,
}

#[derive(Default)]
pub(crate) struct Queue {
    pub tracks: Vec<Track>,
    order: Vec<usize>,
    cursor: usize,
    shuffle: bool,
    pub repeat: Repeat,
    seed: u64,
    ids: Vec<u64>,
    playlist_entries: HashMap<u64, i64>,
    next_id: u64,
    pub snapshot: Arc<QueueSnapshot>,
}

impl Queue {
    pub fn set(&mut self, tracks: Vec<Track>, start: usize) {
        self.set_entries(
            tracks.into_iter().map(|track| (None, track)).collect(),
            start,
        );
    }

    pub fn set_entries(&mut self, entries: Vec<(Option<i64>, Track)>, start: usize) {
        self.playlist_entries.clear();
        self.tracks.clear();
        self.ids.clear();
        let selected = entries
            .get(start.min(entries.len().saturating_sub(1)))
            .map(|(_, track)| track.path.clone());
        let mut seen = HashSet::new();
        for (origin, track) in entries {
            if !seen.insert(track.path.clone()) {
                continue;
            }
            self.next_id += 1;
            self.ids.push(self.next_id);
            if let Some(origin) = origin {
                self.playlist_entries.insert(self.next_id, origin);
            }
            self.tracks.push(track);
        }
        self.order = (0..self.tracks.len()).collect();
        self.cursor = self
            .tracks
            .iter()
            .position(|track| Some(&track.path) == selected.as_ref())
            .unwrap_or(0);
        if self.shuffle {
            self.shuffle_order();
        }
        self.publish();
    }
    fn publish(&mut self) {
        // Manual edits are the new base order; toggling shuffle off restores them.
        if !self.shuffle && self.order.iter().enumerate().any(|(i, &at)| i != at) {
            let mut tracks: Vec<_> = std::mem::take(&mut self.tracks)
                .into_iter()
                .map(Some)
                .collect();
            let ids = std::mem::take(&mut self.ids);
            self.tracks = self
                .order
                .iter()
                .map(|&i| tracks[i].take().unwrap())
                .collect();
            self.ids = self.order.iter().map(|&i| ids[i]).collect();
            self.order = (0..self.tracks.len()).collect();
        }
        self.snapshot = Arc::new(QueueSnapshot {
            entries: self
                .order
                .iter()
                .map(|&i| QueueEntry {
                    id: self.ids[i],
                    track: self.tracks[i].clone(),
                    playlist_entry: self.playlist_entries.get(&self.ids[i]).copied(),
                })
                .collect(),
            current: self.order.get(self.cursor).map(|&i| self.ids[i]),
        });
    }

    pub fn enqueue(&mut self, tracks: Vec<Track>, next: bool) {
        let mut seen: HashSet<_> = self.tracks.iter().map(|track| track.path.clone()).collect();
        let tracks: Vec<_> = tracks
            .into_iter()
            .filter(|track| seen.insert(track.path.clone()))
            .collect();
        if tracks.is_empty() {
            return;
        }
        let start = self.tracks.len();
        let end = start + tracks.len();
        self.tracks.extend(tracks);
        self.ids.extend((start..end).map(|_| {
            self.next_id += 1;
            self.next_id
        }));
        let at = if next && !self.order.is_empty() {
            self.cursor + 1
        } else {
            self.order.len()
        };
        self.order.splice(at..at, start..end);
        self.publish();
    }

    /// Removing other occurrences never restarts the current stream.
    pub fn remove(&mut self, id: u64) {
        let Some(at) = self.order.iter().position(|&i| self.ids[i] == id) else {
            return;
        };
        if at == self.cursor {
            return;
        }
        let removed = self.order.remove(at);
        if at < self.cursor {
            self.cursor -= 1;
        }
        self.tracks.remove(removed);
        self.ids.remove(removed);
        self.playlist_entries.remove(&id);
        for index in &mut self.order {
            if *index > removed {
                *index -= 1;
            }
        }
        self.publish();
    }

    pub fn move_before(&mut self, id: u64, before: Option<u64>) {
        if Some(id) == before {
            return;
        }
        let Some(from) = self.order.iter().position(|&i| self.ids[i] == id) else {
            return;
        };
        if before.is_some_and(|target| !self.order.iter().any(|&i| self.ids[i] == target)) {
            return;
        }
        let current = self.order.get(self.cursor).copied();
        let item = self.order.remove(from);
        let at = before
            .and_then(|target| self.order.iter().position(|&i| self.ids[i] == target))
            .unwrap_or(self.order.len());
        self.order.insert(at, item);
        self.cursor = current
            .and_then(|i| self.order.iter().position(|&j| j == i))
            .unwrap_or(0);
        self.publish();
    }

    pub fn select(&mut self, id: u64) -> bool {
        let Some(at) = self.order.iter().position(|&i| self.ids[i] == id) else {
            return false;
        };
        self.cursor = at;
        self.publish();
        true
    }

    pub fn clear_upcoming(&mut self) {
        self.order.truncate((self.cursor + 1).min(self.order.len()));
        let mut keep = vec![false; self.tracks.len()];
        for &i in &self.order {
            keep[i] = true;
        }
        let ids = std::mem::take(&mut self.ids);
        let mut remap = vec![0; self.tracks.len()];
        for (i, track) in std::mem::take(&mut self.tracks).into_iter().enumerate() {
            if keep[i] {
                remap[i] = self.tracks.len();
                self.tracks.push(track);
                self.ids.push(ids[i]);
            }
        }
        for i in &mut self.order {
            *i = remap[*i];
        }
        let retained: std::collections::HashSet<_> = self.ids.iter().copied().collect();
        self.playlist_entries.retain(|id, _| retained.contains(id));
        self.publish();
    }
    pub fn current(&self) -> Option<&Track> {
        self.order
            .get(self.cursor)
            .and_then(|&i| self.tracks.get(i))
    }
    pub fn shuffle(&mut self, value: bool) {
        let current = self.order.get(self.cursor).copied();
        self.shuffle = value;
        self.order = (0..self.tracks.len()).collect();
        self.cursor = current.unwrap_or(0);
        if value {
            self.shuffle_order();
        }
        self.publish();
    }
    fn shuffle_order(&mut self) {
        // Fisher-Yates permutation: every item exactly once, keep current first.
        self.seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64)
            | 1;
        if self.order.is_empty() {
            return;
        }
        self.order.swap(0, self.cursor);
        self.cursor = 0;
        for i in (2..self.order.len()).rev() {
            self.seed ^= self.seed << 13;
            self.seed ^= self.seed >> 7;
            self.seed ^= self.seed << 17;
            let j = 1 + (self.seed as usize % i);
            self.order.swap(i, j);
        }
    }
    pub fn next(&mut self, automatic: bool) -> bool {
        if self.order.is_empty() {
            return false;
        }
        if automatic && self.repeat == Repeat::Track {
            return true;
        }
        if self.cursor + 1 < self.order.len() {
            self.cursor += 1;
            self.publish();
            true
        } else if self.repeat == Repeat::All {
            if self.shuffle && self.order.len() > 1 {
                // Each round gets a new order, which the track just played
                // does not open.
                self.shuffle_order();
                self.order.rotate_left(1);
            }
            self.cursor = 0;
            self.publish();
            true
        } else {
            false
        }
    }
    pub fn previous(&mut self) -> bool {
        if self.order.is_empty() {
            return false;
        }
        self.cursor = if self.cursor == 0 && self.repeat == Repeat::All {
            self.order.len() - 1
        } else {
            self.cursor.saturating_sub(1)
        };
        self.publish();
        true
    }
}

#[cfg(test)]
#[path = "tests/queue.rs"]
mod tests;
