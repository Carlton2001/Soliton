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
}

pub(crate) enum Discovered {
    File(Candidate),
    Error(String),
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
        move |_| !prune.is_cancelled(),
    );
    for entry in walker {
        if cancel.is_cancelled() {
            break;
        }
        let result = entry.map_err(anyhow::Error::from).and_then(|entry| {
            let path = entry.path();
            if !entry.file_type.is_file() || !audio_path(&path) {
                return Ok(None);
            }
            let metadata = entry
                .metadata
                .ok_or_else(|| anyhow::anyhow!("Missing file metadata"))??;
            let modified = metadata
                .modified()?
                .duration_since(UNIX_EPOCH)?
                .as_nanos()
                .to_string();
            Ok(Some(Candidate {
                path,
                size: metadata.len(),
                modified,
            }))
        });
        let message = match result {
            Ok(Some(file)) => Discovered::File(file),
            Ok(None) => continue,
            Err(error) => Discovered::Error(format!("{error:#}")),
        };
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
