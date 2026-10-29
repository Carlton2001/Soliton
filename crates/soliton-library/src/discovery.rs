//! The only traversal backend. No dependency on dua's CLI or disk-usage model.
use crate::indexer::Cancellation;
use anyhow::Result;
use crossbeam_channel::Sender;
use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub(crate) struct Candidate {
    pub path: PathBuf,
    pub size: u64,
    pub modified: String,
    /// Modification time in seconds; estimates the arrival of the file when the
    /// system records no creation time.
    pub added: i64,
}

pub(crate) enum Discovered {
    File(Candidate),
    /// A directory that could not be listed: only its subtree stays unconcluded.
    Unreadable(PathBuf),
    Error(String),
}

/// Folders the system keeps at a volume root, often unreadable; music never
/// lives there, so they are not traversed.
fn system_folder(entry: &dua_core::Entry) -> bool {
    let name = entry.file_name.to_string_lossy();
    entry.depth > 0 && name == "lost+found"
        || entry.depth > 0
            && (name.eq_ignore_ascii_case("System Volume Information")
                || name.eq_ignore_ascii_case("$RECYCLE.BIN"))
}

pub(crate) fn audio_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "mp3" | "flac" | "m4a" | "mp4" | "aac" | "ogg" | "oga" | "wav" | "aif" | "aiff"
            )
        })
}

pub(crate) fn walk(root: &Path, cancel: &Cancellation, sender: &Sender<Discovered>) -> Result<()> {
    let prune = cancel.clone();
    let walker = dua_core::walk(
        root,
        2,
        dua_core::Order::Completion,
        dua_core::Options::default().queued_batches_per_worker(2),
        move |entry| !prune.is_cancelled() && !system_folder(entry),
    );
    // Walk errors carry no path. Directories that listed no entry and still
    // cannot be read are the failed ones, when they account for every error.
    let mut directories = Vec::new();
    let mut listed = Vec::new();
    let mut errors = Vec::new();
    for entry in walker {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        if let Some(parent) = entry.parent_directory_id {
            let index = parent.index();
            if listed.len() <= index {
                listed.resize(index + 1, false);
            }
            listed[index] = true;
        }
        if let Some(id) = entry.directory_id
            && !system_folder(&entry)
        {
            directories.push((id.index(), entry.path()));
        }
        let result = (|| -> Result<Option<Candidate>> {
            let path = entry.path();
            if !entry.file_type.is_file() || !audio_path(&path) {
                return Ok(None);
            }
            let metadata = entry
                .metadata
                .ok_or_else(|| anyhow::anyhow!("Missing file metadata"))??;
            let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?;
            Ok(Some(Candidate {
                path,
                size: metadata.len(),
                modified: modified.as_nanos().to_string(),
                added: modified.as_secs() as i64,
            }))
        })();
        let message = match result {
            Ok(Some(file)) => Discovered::File(file),
            Ok(None) => continue,
            Err(error) => Discovered::Error(format!("{error:#}")),
        };
        if !send(sender, message, cancel) {
            return Ok(());
        }
    }
    if errors.is_empty() {
        return Ok(());
    }
    let unreadable: Vec<_> = directories
        .into_iter()
        .filter(|(id, path)| {
            !listed.get(*id).copied().unwrap_or(false) && std::fs::read_dir(path).is_err()
        })
        .map(|(_, path)| path)
        .collect();
    let messages: Vec<_> = if unreadable.len() == errors.len() {
        unreadable.into_iter().map(Discovered::Unreadable).collect()
    } else {
        errors
            .into_iter()
            .map(|error| Discovered::Error(error.to_string()))
            .collect()
    };
    for message in messages {
        if !send(sender, message, cancel) {
            break;
        }
    }
    Ok(())
}

pub(crate) fn send<T>(sender: &Sender<T>, mut message: T, cancel: &Cancellation) -> bool {
    loop {
        if cancel.is_cancelled() {
            return false;
        }
        match sender.send_timeout(message, std::time::Duration::from_millis(30)) {
            Ok(()) => return true,
            Err(crossbeam_channel::SendTimeoutError::Timeout(value)) => message = value,
            Err(_) => return false,
        }
    }
}
