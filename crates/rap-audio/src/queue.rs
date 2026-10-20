use rap_core::model::Track;

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
}

impl Queue {
    pub fn set(&mut self, tracks: Vec<Track>, start: usize) {
        self.tracks = tracks;
        self.order = (0..self.tracks.len()).collect();
        self.cursor = start.min(self.tracks.len().saturating_sub(1));
        if self.shuffle {
            self.shuffle_order();
        }
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
            true
        } else if self.repeat == Repeat::All {
            self.cursor = 0;
            true
        } else {
            false
        }
    }
    pub fn previous(&mut self) -> bool {
        if self.order.is_empty() {
            return false;
        }
        self.cursor = self.cursor.saturating_sub(1);
        true
    }
}

#[cfg(test)]
#[path = "tests/queue.rs"]
mod tests;
