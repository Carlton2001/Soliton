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
        volume_serial(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        None
    }
}

/// Serial number of the volume holding the directory, through its own handle:
/// a volume mounted in an NTFS folder, or another disk given the same drive
/// letter, has its own.
#[cfg(windows)]
fn volume_serial(path: &Path) -> Option<u64> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, GetFileInformationByHandle,
    };
    // Directories open only with backup semantics; querying needs no access right.
    let directory = std::fs::OpenOptions::new()
        .access_mode(0)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .ok()?;
    // SAFETY: plain data, fully written by a successful call.
    let mut information: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: the handle stays open during the call and the pointer is valid.
    let found = unsafe { GetFileInformationByHandle(directory.as_raw_handle(), &mut information) };
    (found != 0).then(|| u64::from(information.dwVolumeSerialNumber))
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
