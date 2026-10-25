//! Independent audio engine. Commands and events are connected by the application.
mod decoder;
mod devices;
mod engine;
mod output;
mod queue;
mod resample;
/// Fixtures shared by the decoder and engine tests.
#[cfg(test)]
#[path = "tests/support.rs"]
mod test_support;

pub use devices::{OutputDevice, OutputInfo};
pub use engine::Engine;
pub use soliton_core::config::Repeat;
use soliton_core::{failure::Failure, model::Track};
pub use soliton_network::{Device as NetworkDevice, Protocol as NetworkProtocol};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
    Failed,
}

/// An occurrence, so the same track may appear several times in a session.
#[derive(Clone, Debug)]
pub struct QueueEntry {
    pub id: u64,
    pub track: Track,
    /// Saved playlist occurrence that started this entry, if any.
    pub playlist_entry: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct QueueSnapshot {
    pub entries: Vec<QueueEntry>,
    pub current: Option<u64>,
}

impl QueueSnapshot {
    pub fn current_playlist_entry(&self) -> Option<i64> {
        self.entries
            .iter()
            .find(|e| Some(e.id) == self.current)
            .and_then(|e| e.playlist_entry)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Playback {
    pub queue: std::sync::Arc<QueueSnapshot>,
    pub state: PlaybackState,
    pub track: Option<Track>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: Repeat,
    /// None selects the system default at the next stream opening.
    pub selected_output: Option<String>,
    pub outputs: std::sync::Arc<Vec<OutputDevice>>,
    pub outputs_error: Option<Failure>,
    pub output: Option<OutputInfo>,
    pub network_outputs: std::sync::Arc<Vec<NetworkDevice>>,
    pub network_output: Option<NetworkDevice>,
    pub network_enabled: bool,
    pub network_discovering: bool,
    pub network_error: Option<Failure>,
    /// Discovery/selection errors do not interrupt an existing working stream.
    pub output_error: Option<Failure>,
    pub output_changing: bool,
    pub error: Option<Failure>,
    /// Latest problem playback went past: skipped passages or unreadable tracks.
    pub warning: Option<Failure>,
}

pub enum Command {
    PlayPlaylist {
        entries: Vec<(i64, Track)>,
        start: usize,
    },
    /// Restores a saved queue paused, in its saved order even with shuffle on.
    LoadSelection {
        entries: Vec<(Option<i64>, Track)>,
        start: usize,
        position_ms: u64,
    },
    Enqueue {
        tracks: Vec<Track>,
        next: bool,
    },
    RemoveQueued(u64),
    MoveQueued {
        id: u64,
        before: Option<u64>,
    },
    PlayQueued(u64),
    ClearUpcoming,
    Play {
        tracks: Vec<Track>,
        start: usize,
    },
    /// Prepares a queue paused at a position, without opening any output:
    /// the next [`Command::Resume`] starts there.
    Load {
        tracks: Vec<Track>,
        start: usize,
        position_ms: u64,
    },
    Resume,
    Pause,
    Stop,
    Seek(u64),
    Next,
    Previous,
    Volume(f32),
    Shuffle(bool),
    Repeat(Repeat),
    SelectOutput(Option<String>),
    RefreshOutputs,
    DiscoverNetworkOutputs,
    SelectNetworkOutput(String),
}
