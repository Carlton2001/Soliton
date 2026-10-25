//! User selections survive unavailable sources and explicit catalogue removal.
use crate::model::{Metadata, Track};

#[derive(Clone, Debug)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub entries: Vec<PlaylistEntry>,
}

#[derive(Clone, Debug)]
pub struct PlaylistEntry {
    pub id: i64,
    pub track_id: Option<i64>,
    /// Last known tags, retained when a source is removed from the catalogue.
    pub metadata: Metadata,
}

pub enum PlaylistChange {
    Create {
        name: String,
        tracks: Vec<Track>,
    },
    Rename {
        id: i64,
        name: String,
    },
    Delete(i64),
    Append {
        id: i64,
        tracks: Vec<Track>,
    },
    Remove {
        id: i64,
        entry: i64,
    },
    Move {
        id: i64,
        entry: i64,
        before: Option<i64>,
    },
}
