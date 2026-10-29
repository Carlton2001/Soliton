//! Reachability checks that cannot stall the caller on a hung network share.
use crossbeam_channel::bounded;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

/// Identifier of the volume holding `path`, where the system exposes one. An
/// unmounted mount point leaves its folder on the parent volume.
pub(crate) fn volume(path: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(path).ok().map(|metadata| metadata.dev())
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let canonical = std::fs::canonicalize(path).ok()?;
        match canonical.components().next()? {
            Component::Prefix(prefix) => match prefix.kind() {
                Prefix::VerbatimDisk(letter) | Prefix::Disk(letter) => {
                    Some(u64::from(letter.to_ascii_uppercase()))
                }
                _ => None,
            },
            _ => None,
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        None
    }
}

/// An empty source root is a deliberate deletion only when it is still on the
/// volume that held its music; otherwise it may be an unmounted mount point.
pub(crate) fn emptied_in_place(recorded: Option<u64>, current: Option<u64>) -> bool {
    recorded.is_some() && recorded == current
}

/// At most one probe thread per path: a share that hangs keeps a single thread
/// waiting on the system, never one per check.
#[derive(Clone, Default)]
pub struct Prober {
    in_flight: Arc<Mutex<HashSet<PathBuf>>>,
}

impl Prober {
    /// `Some(reachable)` when the directory answered within `timeout`; `None` when
    /// it did not, or when an earlier probe of it is still waiting.
    pub fn reachable(&self, path: &Path, timeout: Duration) -> Option<bool> {
        if !self.lock().insert(path.to_path_buf()) {
            return None;
        }
        let (sender, receiver) = bounded(1);
        let in_flight = self.in_flight.clone();
        let target = path.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("soliton-probe".into())
            .spawn(move || {
                let reachable = std::fs::read_dir(&target).is_ok();
                in_flight
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&target);
                let _ = sender.send(reachable);
            });
        if spawned.is_err() {
            self.lock().remove(path);
            return None;
        }
        receiver.recv_timeout(timeout).ok()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashSet<PathBuf>> {
        self.in_flight.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
#[path = "tests/availability.rs"]
mod tests;
