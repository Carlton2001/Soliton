//! Independent audio engine. Commands and events are connected by the application.
mod decoder;
mod devices;
mod engine;
mod output;
mod queue;
mod resample;

pub use devices::{OutputDevice, OutputInfo};
pub use engine::Engine;
pub use queue::Repeat;
use rap_core::model::Track;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
    Failed,
}

#[derive(Clone, Debug, Default)]
pub struct Playback {
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
    pub outputs_error: Option<String>,
    pub output: Option<OutputInfo>,
    /// Discovery/selection errors do not interrupt an existing working stream.
    pub output_error: Option<String>,
    pub error: Option<String>,
}

pub enum Command {
    Play { tracks: Vec<Track>, start: usize },
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
}
