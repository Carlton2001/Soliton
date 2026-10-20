//! Files played once, from a drop or the open dialog. They are read like indexed
//! files but never stored in the library database.
use crate::{Cancellation, discovery::audio_path, metadata};
use rap_core::model::Track;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct LooseFiles {
    /// In album order within each folder; identifiers are negative so they never
    /// match library tracks.
    pub tracks: Vec<Track>,
    /// Bounded sample of unreadable entries.
    pub errors: Vec<String>,
    /// More audio files than `limit` were found; the first ones are kept.
    pub truncated: bool,
}

/// Expands folders recursively and reads tags. Covers go to `covers`, a cache
/// directory the caller owns. Blocking: call from a background thread.
pub fn read_loose_files(
    paths: &[PathBuf],
    covers: &Path,
    cancel: &Cancellation,
    limit: usize,
) -> LooseFiles {
    let mut result = LooseFiles::default();
    let mut files = Vec::new();
    for path in paths {
        collect(path, &mut files, &mut result, cancel, limit);
    }
    if std::fs::create_dir_all(covers).is_err() {
        result
            .errors
            .push(format!("{}: cannot create", covers.display()));
    }
    for (index, path) in files.into_iter().enumerate() {
        if cancel.is_cancelled() {
            break;
        }
        match metadata::read(&path, covers) {
            Ok(value) => result.tracks.push(Track {
                id: -(index as i64) - 1,
                source_id: 0,
                path,
                metadata: value.metadata,
                cover: value.cover,
                available: true,
            }),
            Err(error) => error_sample(&mut result, format!("{}: {error:#}", path.display())),
        }
    }
    result.tracks.sort_by_cached_key(|track| {
        (
            track.path.parent().map(Path::to_path_buf),
            track.metadata.disc.unwrap_or(1),
            track.metadata.track.unwrap_or(u32::MAX),
            track.path.clone(),
        )
    });
    result
}

fn collect(
    path: &Path,
    files: &mut Vec<PathBuf>,
    result: &mut LooseFiles,
    cancel: &Cancellation,
    limit: usize,
) {
    if cancel.is_cancelled() || result.truncated {
        return;
    }
    // Links are not followed, as during indexing.
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        error_sample(result, format!("{}: unreadable", path.display()));
        return;
    };
    if metadata.is_file() {
        if audio_path(path) {
            if files.len() >= limit {
                result.truncated = true;
            } else {
                files.push(path.to_path_buf());
            }
        }
        return;
    }
    if !metadata.is_dir() {
        return;
    }
    match std::fs::read_dir(path) {
        Ok(entries) => {
            let mut children: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
            children.sort();
            for child in children {
                collect(&child, files, result, cancel, limit);
            }
        }
        Err(error) => error_sample(result, format!("{}: {error}", path.display())),
    }
}

fn error_sample(result: &mut LooseFiles, text: String) {
    if result.errors.len() < 16 {
        result.errors.push(text);
    }
}

#[cfg(test)]
#[path = "tests/standalone.rs"]
mod tests;
