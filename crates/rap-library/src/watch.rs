//! Filesystem notifications for source folders, coalesced before rescanning.
//!
//! Notifications complement scans; they are not trusted alone. Network shares may
//! drop them, so the application also runs periodic incremental scans.
use crate::discovery::audio_path;
use anyhow::Result;
use crossbeam_channel::Sender;
use notify::{EventKind, RecursiveMode, Watcher as _};
use rap_core::model::Source;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub enum WatchEvent {
    Paths(Vec<PathBuf>),
    /// Events were lost; every watched source must be rescanned.
    Overflow,
}

pub struct SourceWatcher {
    inner: notify::RecommendedWatcher,
    watched: HashMap<i64, PathBuf>,
    overflow: Arc<AtomicBool>,
}

impl SourceWatcher {
    /// The handler runs on notify's thread and never blocks: when `events` is full,
    /// the loss is recorded for [`Self::overflowed`] instead.
    pub fn new(events: Sender<WatchEvent>) -> Result<Self> {
        let overflow = Arc::new(AtomicBool::new(false));
        let lost = overflow.clone();
        let inner = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
            let message = match event {
                Ok(event) if event.need_rescan() => WatchEvent::Overflow,
                Ok(event) if matches!(event.kind, EventKind::Access(_)) => return,
                Ok(event) => WatchEvent::Paths(event.paths),
                Err(_) => WatchEvent::Overflow,
            };
            if let Err(crossbeam_channel::TrySendError::Full(_)) = events.try_send(message) {
                lost.store(true, Ordering::Relaxed);
            }
        })?;
        Ok(Self {
            inner,
            watched: HashMap::new(),
            overflow,
        })
    }

    /// Whether events were dropped since the previous call.
    pub fn overflowed(&self) -> bool {
        self.overflow.swap(false, Ordering::Relaxed)
    }

    /// Watches available sources and stops watching the others. Call only with
    /// sources just confirmed reachable: registering an offline share may block.
    /// Returns per-source failures; scans still cover those sources.
    pub fn sync(&mut self, sources: &[Source]) -> Vec<String> {
        let mut errors = Vec::new();
        let wanted: HashMap<_, _> = sources
            .iter()
            .filter(|source| source.available)
            .map(|source| (source.id, &source.path))
            .collect();
        self.watched.retain(|id, path| {
            let keep = wanted.get(id).is_some_and(|wanted| *wanted == path);
            if !keep {
                let _ = self.inner.unwatch(path);
            }
            keep
        });
        for (id, path) in wanted {
            if self.watched.contains_key(&id) {
                continue;
            }
            match self.inner.watch(path, RecursiveMode::Recursive) {
                Ok(()) => {
                    self.watched.insert(id, path.clone());
                }
                Err(error) => errors.push(format!("{}: {error}", path.display())),
            }
        }
        errors
    }
}

/// Groups bursts of notifications (copies, tag editors, extractions) into one batch.
pub struct Coalescer {
    paths: BTreeSet<PathBuf>,
    overflow: bool,
    first: Option<Instant>,
    last: Option<Instant>,
    quiet: Duration,
    longest: Duration,
}

/// Beyond this many distinct paths a full rescan is cheaper and bounds memory.
const MAX_PATHS: usize = 4096;

impl Coalescer {
    /// Waits for `quiet` without events, but never more than `longest` in total.
    pub fn new(quiet: Duration, longest: Duration) -> Self {
        Self {
            paths: BTreeSet::new(),
            overflow: false,
            first: None,
            last: None,
            quiet,
            longest,
        }
    }

    pub fn push(&mut self, event: WatchEvent, now: Instant) {
        match event {
            WatchEvent::Overflow => self.overflow = true,
            WatchEvent::Paths(paths) => {
                self.paths
                    .extend(paths.into_iter().filter(|path| relevant(path)));
                if self.paths.len() > MAX_PATHS {
                    self.overflow = true;
                }
            }
        }
        if self.overflow || !self.paths.is_empty() {
            self.first.get_or_insert(now);
            self.last = Some(now);
        }
        if self.overflow {
            self.paths.clear();
        }
    }

    /// When the pending batch becomes ready, if any.
    pub fn deadline(&self) -> Option<Instant> {
        Some((self.last? + self.quiet).min(self.first? + self.longest))
    }

    /// The pending batch once ready: changed paths, or `None` paths for a full rescan.
    pub fn take(&mut self, now: Instant) -> Option<Option<Vec<PathBuf>>> {
        if self.deadline()? > now {
            return None;
        }
        self.first = None;
        self.last = None;
        let paths = std::mem::take(&mut self.paths);
        Some(if std::mem::take(&mut self.overflow) {
            None
        } else {
            Some(paths.into_iter().collect())
        })
    }
}

/// Audio files and directories matter; other existing files (artwork, playlists,
/// temporary downloads) cannot change the catalogue. Vanished paths may have been
/// either, so they are kept.
fn relevant(path: &Path) -> bool {
    audio_path(path) || std::fs::metadata(path).map_or(true, |metadata| metadata.is_dir())
}

/// Groups changed paths by source, as directories to rescan.
pub fn scopes(sources: &[Source], paths: &[PathBuf]) -> Vec<(i64, Vec<PathBuf>)> {
    let mut scopes = BTreeMap::<i64, BTreeSet<PathBuf>>::new();
    for path in paths {
        let Some(source) = sources
            .iter()
            .filter(|source| path.starts_with(&source.path))
            .max_by_key(|source| source.path.components().count())
        else {
            continue;
        };
        // Files are rescanned through their folder; vanished entries through their
        // parent. Scanning resolves missing directories to existing ancestors.
        let dir = if path.is_dir() {
            path.clone()
        } else {
            path.parent().unwrap_or(&source.path).to_path_buf()
        };
        scopes.entry(source.id).or_default().insert(dir);
    }
    scopes
        .into_iter()
        .map(|(id, dirs)| (id, dirs.into_iter().collect()))
        .collect()
}

#[cfg(test)]
#[path = "tests/watch.rs"]
mod tests;
