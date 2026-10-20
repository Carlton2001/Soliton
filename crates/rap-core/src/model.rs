use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Metadata {
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub disc: Option<u32>,
    pub duration_ms: u64,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
    pub format: String,
    /// Names of fields inferred from the path rather than embedded tags.
    pub inferred: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Track {
    pub id: i64,
    pub source_id: i64,
    pub path: PathBuf,
    pub metadata: Metadata,
    pub cover: Option<PathBuf>,
    pub available: bool,
}

#[derive(Clone, Debug)]
pub struct Source {
    pub id: i64,
    pub path: PathBuf,
    pub available: bool,
}

#[derive(Clone, Debug)]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub tracks: Vec<Track>,
}

impl Album {
    pub fn cover(&self) -> Option<&PathBuf> {
        self.tracks.iter().find_map(|track| track.cover.as_ref())
    }
    pub fn duration_ms(&self) -> u64 {
        self.tracks
            .iter()
            .map(|track| track.metadata.duration_ms)
            .sum()
    }
}

#[derive(Clone, Debug, Default)]
pub struct LibrarySnapshot {
    pub sources: Vec<Source>,
    pub albums: Vec<Album>,
}
