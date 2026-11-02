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

/// Longest playlist name, in characters.
pub const MAX_NAME_CHARS: usize = 200;

/// Names differing only by case or surrounding spaces designate the same playlist.
pub fn same_name(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// `base`, or `base (2)`, `base (3)`… for the first one no existing name takes.
pub fn unique_name<'a>(base: &str, names: impl Iterator<Item = &'a str> + Clone) -> String {
    let base = base.trim();
    let free = |name: &str| !names.clone().any(|taken| same_name(taken, name));
    if free(base) {
        return base.to_owned();
    }
    (2..)
        .map(|number| {
            let suffix = format!(" ({number})");
            let room = MAX_NAME_CHARS.saturating_sub(suffix.chars().count());
            let start: String = base.chars().take(room).collect();
            format!("{}{suffix}", start.trim_end())
        })
        .find(|name| free(name))
        .unwrap_or_default()
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
    /// Gathers the entries before `before`, in the given order.
    MoveBlock {
        id: i64,
        entries: Vec<i64>,
        before: Option<i64>,
    },
}

#[cfg(test)]
#[path = "tests/playlist.rs"]
mod tests;
