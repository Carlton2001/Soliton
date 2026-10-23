//! Pure rules assigning tracks to editions and editions to albums.
//!
//! An edition is one album folder of a source, disc subfolders included. Editions
//! from different folders join one album only when the album artist, normalised
//! title, compilation status and year are identical, the year being present.
//! Titles inferred from folder names never merge. Ambiguity keeps albums apart.
use rap_core::model::Metadata;
pub(crate) use rap_core::text::fold;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Disc number of folders such as `CD1`, `Disc 2`, `disque_3` or `CD 2 - Bonus`.
pub(crate) fn disc_folder(name: &str) -> Option<u32> {
    let lower = name.trim().to_lowercase();
    let rest = ["disque", "disc", "disk", "cd", "dvd"]
        .iter()
        .find_map(|prefix| lower.strip_prefix(prefix))?;
    let rest = rest.trim_start_matches([' ', '_', '-', '.']);
    let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits == 0 || digits > 3 {
        return None;
    }
    let tail = &rest[digits..];
    if !tail.is_empty() && !tail.starts_with([' ', '_', '-', '.', '(', '[']) {
        return None;
    }
    rest[..digits].parse().ok()
}

/// Folder holding the album, skipping one disc folder level.
pub(crate) fn album_folder(path: &Path) -> Option<&Path> {
    let parent = path.parent()?;
    match parent.file_name() {
        Some(name) if disc_folder(&name.to_string_lossy()).is_some() => parent.parent(),
        _ => Some(parent),
    }
}

/// Like [`album_folder`], without leaving the source root.
pub(crate) fn edition_folder<'a>(path: &'a Path, root: &Path) -> &'a Path {
    let parent = path.parent().unwrap_or(path);
    match album_folder(path) {
        Some(folder) if folder.starts_with(root) => folder,
        _ => parent,
    }
}

fn tagged_album_artist(metadata: &Metadata) -> &str {
    if metadata.is_inferred("album_artist") {
        ""
    } else {
        &metadata.album_artist
    }
}

/// Stable per-track key of the edition. Inferred album artists are ignored so that
/// untagged compilations stay one edition.
pub(crate) fn edition_identity(source: i64, folder: &str, metadata: &Metadata) -> String {
    edition_identity_with_artist(
        source,
        folder,
        &metadata.album,
        tagged_album_artist(metadata),
    )
}

pub(crate) fn edition_identity_with_artist(
    source: i64,
    folder: &str,
    album: &str,
    artist: &str,
) -> String {
    serde_json::json!([source, folder, fold(album), fold(artist)]).to_string()
}

/// Fill missing album-artist identities only when this album in this folder has
/// one explicit album artist and every untagged track agrees with that artist.
/// Never change the metadata itself or guess for compilations/conflicting tags.
pub(crate) fn compatible_album_artist<'a>(
    tracks: impl IntoIterator<Item = &'a Metadata>,
) -> Option<String> {
    let tracks: Vec<_> = tracks.into_iter().collect();
    if tracks
        .iter()
        .any(|track| track.compilation || track.is_inferred("album"))
    {
        return None;
    }
    let explicit: BTreeSet<_> = tracks
        .iter()
        .map(|track| fold(tagged_album_artist(track)))
        .filter(|artist| !artist.is_empty())
        .collect();
    if explicit.len() != 1 {
        return None;
    }
    let artist = explicit.into_iter().next()?;
    tracks
        .iter()
        .filter(|track| tagged_album_artist(track).is_empty())
        .all(|track| !track.is_inferred("artist") && fold(&track.artist) == artist)
        .then_some(artist)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AlbumAssignment {
    pub identity: String,
    pub title: String,
    pub artist: String,
    pub compilation: bool,
}

/// Album of one edition, from its tracks' metadata (ordered by track id).
pub(crate) fn assign<'a>(
    edition_identity: &str,
    tracks: impl IntoIterator<Item = &'a Metadata>,
) -> Option<AlbumAssignment> {
    let tracks: Vec<_> = tracks.into_iter().collect();
    let first = tracks.first()?;
    let flagged = tracks.iter().any(|m| m.compilation);
    let tagged = tracks
        .iter()
        .map(|m| tagged_album_artist(m))
        .find(|name| !name.is_empty());
    let track_artists: BTreeSet<_> = tracks
        .iter()
        .filter(|m| !m.artist.is_empty())
        .map(|m| fold(&m.artist))
        .collect();
    let (artist, compilation) = match tagged {
        Some(name) => (name.to_owned(), flagged),
        None if flagged || track_artists.len() > 1 => (String::new(), true),
        None => (first.album_artist.clone(), false),
    };
    let year = most_common_year(&tracks);
    let inferred_title = tracks.iter().any(|m| m.is_inferred("album"));
    let identity = match year {
        Some(year) if !inferred_title => serde_json::json!([
            "album",
            fold(&artist),
            compilation,
            fold(&first.album),
            year
        ])
        .to_string(),
        _ => serde_json::json!(["edition", edition_identity]).to_string(),
    };
    Some(AlbumAssignment {
        identity,
        title: first.album.clone(),
        artist,
        compilation,
    })
}

/// Most frequent year, the earliest on ties; compilations often vary per track.
fn most_common_year(tracks: &[&Metadata]) -> Option<u32> {
    let mut counts = BTreeMap::<u32, usize>::new();
    for year in tracks.iter().filter_map(|m| m.year) {
        *counts.entry(year).or_default() += 1;
    }
    let best = counts.values().copied().max()?;
    counts
        .into_iter()
        .find_map(|(year, count)| (count == best).then_some(year))
}

#[cfg(test)]
#[path = "tests/grouping.rs"]
mod tests;
