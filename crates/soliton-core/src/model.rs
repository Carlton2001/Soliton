use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Metadata {
    pub title: String,
    pub artist: String,
    pub album_artist: String,
    pub album: String,
    pub genre: String,
    pub year: Option<u32>,
    pub track: Option<u32>,
    pub disc: Option<u32>,
    /// Compilation flag embedded in tags (ID3 TCMP, MP4 cpil, Vorbis COMPILATION).
    pub compilation: bool,
    pub duration_ms: u64,
    pub sample_rate: Option<u32>,
    pub bit_depth: Option<u8>,
    pub channels: Option<u8>,
    pub format: String,
    /// Names of fields inferred from the path rather than embedded tags.
    pub inferred: Vec<String>,
}

impl Metadata {
    pub fn is_inferred(&self, field: &str) -> bool {
        self.inferred.iter().any(|name| name == field)
    }
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

/// One physical copy of an album, usually one folder of a source.
#[derive(Clone, Debug)]
pub struct Edition {
    pub id: i64,
    pub source_id: i64,
    pub folder: PathBuf,
    pub tracks: Vec<Track>,
}

impl Edition {
    pub fn cover(&self) -> Option<&PathBuf> {
        self.tracks.iter().find_map(|track| track.cover.as_ref())
    }
    pub fn duration_ms(&self) -> u64 {
        self.tracks
            .iter()
            .map(|track| track.metadata.duration_ms)
            .sum()
    }
    pub fn available(&self) -> bool {
        self.tracks.iter().any(|track| track.available)
    }
}

#[derive(Clone, Debug)]
pub struct Album {
    pub id: i64,
    pub title: String,
    /// Album artist; empty for a compilation without an album artist tag.
    pub artist: String,
    pub compilation: bool,
    /// Latest arrival of one of its files, in seconds since the epoch.
    pub added: i64,
    /// Never empty. Ordered best first: available, then highest resolution.
    pub editions: Vec<Edition>,
}

impl Album {
    pub fn primary(&self) -> &Edition {
        &self.editions[0]
    }
    pub fn cover(&self) -> Option<&PathBuf> {
        self.editions.iter().find_map(Edition::cover)
    }
    pub fn tracks(&self) -> impl Iterator<Item = &Track> {
        self.editions.iter().flat_map(|edition| &edition.tracks)
    }
    /// Whether `field` comes from folder and file names rather than tags.
    pub fn inferred(&self, field: &str) -> bool {
        self.primary()
            .tracks
            .first()
            .is_some_and(|track| track.metadata.is_inferred(field))
    }
    /// Whether the album artist comes from folder names. Copied from the
    /// track artist tag, it is still a tag: only grouping treats it as inferred.
    pub fn artist_inferred(&self) -> bool {
        self.inferred("album_artist") && self.inferred("artist")
    }
}

#[derive(Clone, Debug, Default)]
pub struct LibrarySnapshot {
    pub sources: Vec<Source>,
    pub albums: Vec<Album>,
}

#[cfg(test)]
#[path = "tests/model.rs"]
mod tests;
