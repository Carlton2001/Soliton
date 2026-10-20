//! Reachability checks that cannot stall the caller on a hung network share.
use crossbeam_channel::bounded;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

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
            .name("rap-probe".into())
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
