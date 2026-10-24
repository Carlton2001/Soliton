use crate::{
    Library,
    catalog::{Known, Scope, regroup},
    discovery::{self, Candidate, Discovered},
    metadata,
};
use anyhow::{Result, bail};
use crossbeam_channel::bounded;
use rap_core::failure::{Failure, Kind};
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Clone, Default)]
pub struct Cancellation(Arc<AtomicBool>);
impl Cancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ScanProgress {
    pub discovered: usize,
    pub indexed: usize,
    pub unchanged: usize,
    pub errors: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ScanReport {
    pub progress: ScanProgress,
    pub cancelled: bool,
    /// The whole scope was traversed without directory errors.
    pub complete: bool,
    /// Entries forgotten because their file disappeared from an accessible source.
    pub removed: usize,
    /// Bounded diagnostic sample; total errors are counted separately.
    pub diagnostics: Vec<Failure>,
}
impl ScanReport {
    fn error(&mut self, failure: Failure) {
        self.progress.errors += 1;
        if self.diagnostics.len() < 32 {
            self.diagnostics.push(failure);
        }
    }
}

/// Number of stored files between commits; each commit makes progress visible.
const BATCH: usize = 64;

/// Short writes a scan lets through on its connection, between two batches, so
/// they wait for one batch rather than a whole scan.
pub trait Interlude {
    /// Whether writes are waiting; the scan then commits its batch early.
    fn waiting(&self) -> bool;
    /// Runs outside any scan transaction. Must not scan.
    fn run(&mut self, library: &mut Library);
}

/// No write to let through.
impl Interlude for () {
    fn waiting(&self) -> bool {
        false
    }
    fn run(&mut self, _: &mut Library) {}
}

impl Library {
    /// Synchronous service API. Invoke from a background worker, never GPUI.
    pub fn scan(
        &mut self,
        source_id: i64,
        cancel: &Cancellation,
        progress: impl FnMut(&ScanProgress),
    ) -> Result<ScanReport> {
        self.scan_scope(source_id, &[], cancel, progress, &mut ())
    }

    /// Scans directories of a source, or the whole source when `dirs` is empty.
    /// Missing directories are replaced by their nearest existing ancestor.
    pub fn scan_scope(
        &mut self,
        source_id: i64,
        dirs: &[PathBuf],
        cancel: &Cancellation,
        mut progress: impl FnMut(&ScanProgress),
        interlude: &mut dyn Interlude,
    ) -> Result<ScanReport> {
        let source = self
            .sources()?
            .into_iter()
            .find(|s| s.id == source_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown source"))?;
        if let Err(error) = std::fs::read_dir(&source.path) {
            self.set_source_available(source_id, false)?;
            let failure = Failure::with(Kind::SourceInaccessible, error.to_string());
            return Err(failure.about(source.path.display()).into());
        }
        let dirs = resolve_scope(&source.path, dirs);
        self.connection.execute(
            "UPDATE sources SET available=1,generation=generation+1 WHERE id=?1",
            [source_id],
        )?;
        let generation: i64 = self.connection.query_row(
            "SELECT generation FROM sources WHERE id=?1",
            [source_id],
            |r| r.get(0),
        )?;
        let covers = self.covers.clone();
        let root = source.path.clone();
        let mut report = ScanReport::default();
        let mut traversal_failed = false;
        std::thread::scope(|scope| -> Result<()> {
            let (found_tx, found_rx) = bounded(64);
            let (jobs_tx, jobs_rx) = bounded::<Candidate>(16);
            let (results_tx, results_rx) = bounded(16);
            let scan_cancel = cancel.clone();
            let walked = &dirs;
            scope.spawn(move || {
                for dir in walked {
                    if let Err(error) = discovery::walk(dir, &scan_cancel, &found_tx) {
                        discovery::send(
                            &found_tx,
                            Discovered::Error(format!("{error:#}")),
                            &scan_cancel,
                        );
                    }
                }
            });
            let readers = std::thread::available_parallelism()
                .map_or(2, usize::from)
                .clamp(1, 4);
            for _ in 0..readers {
                let jobs = jobs_rx.clone();
                let results = results_tx.clone();
                let covers = covers.clone();
                scope.spawn(move || {
                    while let Ok(file) = jobs.recv() {
                        if cancel.is_cancelled() {
                            break;
                        }
                        let value = metadata::read(&file.path, &covers);
                        if !discovery::send(&results, (file, value), cancel) {
                            break;
                        }
                    }
                });
            }
            drop(jobs_rx);
            drop(results_tx);
            let mut jobs = Some(jobs_tx);
            let mut found = Some(found_rx);
            let mut pending = None;
            let mut dirty = BTreeSet::new();
            let mut covers_present = HashMap::new();
            // One owner writes SQLite in bounded batches, also consuming results while
            // jobs are full. Blocking on a full jobs queue here would deadlock readers.
            self.connection.execute_batch("BEGIN IMMEDIATE")?;
            let outcome = (|| -> Result<()> {
                let never = crossbeam_channel::never();
                let mut batched = 0;
                loop {
                    if cancel.is_cancelled() {
                        break;
                    }
                    if let Some(file) = pending.take() {
                        match jobs.as_ref().unwrap().try_send(file) {
                            Ok(()) => {}
                            Err(crossbeam_channel::TrySendError::Full(file)) => {
                                pending = Some(file)
                            }
                            Err(_) => bail!("Metadata workers disconnected"),
                        }
                    }
                    let found_queue = if pending.is_none() {
                        found.as_ref().unwrap_or(&never)
                    } else {
                        &never
                    };
                    crossbeam_channel::select! {
                        recv(found_queue) -> message => match message {
                            // SQLite stores paths as text: such a file is reported and
                            // skipped, without stopping the scan or its conclusions.
                            Ok(Discovered::File(file)) if file.path.to_str().is_none() => {
                                report.progress.discovered += 1;
                                report.error(
                                    Failure::new(Kind::PathNotUnicode).about(file.path.display()),
                                );
                            }
                            Ok(Discovered::File(file)) => {
                                report.progress.discovered += 1;
                                match self.known(source_id, &file.path, file.size, &file.modified, &mut covers_present)? {
                                    Known::Unchanged | Known::Failed => {
                                        self.seen(source_id, &file.path, generation)?;
                                        report.progress.unchanged += 1;
                                    }
                                    Known::Changed => pending = Some(file),
                                }
                            }
                            Ok(Discovered::Error(error)) => {
                                traversal_failed = true;
                                report.error(Failure::with(Kind::FolderUnreadable, error));
                            }
                            Err(_) => { found = None; jobs.take(); }
                        },
                        recv(results_rx) -> message => match message {
                            Ok((file, value)) => {
                                self.seen(source_id, &file.path, generation)?;
                                match value {
                                    Ok(value) => {
                                        let editions = self.store(source_id, &root, &file, generation, &value)?;
                                        dirty.extend(editions.into_iter().flatten());
                                        report.progress.indexed += 1;
                                        for warning in value.warnings {
                                            report.error(Failure::with(Kind::CoverIgnored, warning));
                                        }
                                    }
                                    Err(error) => {
                                        self.record_failure(source_id, &file.path, file.size, &file.modified, generation)?;
                                        report.error(
                                            Failure::with(Kind::UnreadableFile, format!("{error:#}"))
                                                .about(file.path.display()),
                                        );
                                    }
                                }
                                batched += 1;
                            }
                            Err(_) => break,
                        },
                        default(std::time::Duration::from_millis(30)) => {},
                    }
                    let waiting = interlude.waiting();
                    if batched >= BATCH || waiting {
                        regroup(&self.connection, &std::mem::take(&mut dirty))?;
                        self.connection.execute_batch("COMMIT")?;
                        if waiting {
                            interlude.run(self);
                        }
                        self.connection.execute_batch("BEGIN IMMEDIATE")?;
                        batched = 0;
                    }
                    progress(&report.progress);
                }
                // Albums stay consistent with stored tracks even after cancellation.
                regroup(&self.connection, &dirty)
            })();
            // Drop receivers before joining workers, including after SQLite errors.
            drop(found);
            drop(jobs);
            drop(results_rx);
            match outcome {
                Ok(()) => self.connection.execute_batch("COMMIT")?,
                Err(error) => {
                    self.connection.execute_batch("ROLLBACK")?;
                    return Err(error);
                }
            }
            Ok(())
        })?;
        report.cancelled = cancel.is_cancelled();
        // A partial traversal or a source lost meanwhile never establishes disappearance.
        report.complete =
            !report.cancelled && !traversal_failed && std::fs::read_dir(&source.path).is_ok();
        if report.complete {
            let scope = Scope {
                source: source_id,
                root: &source.path,
                dirs: &dirs,
            };
            // An empty root more likely means an unmounted volume than a deleted library.
            if scope.whole() && report.progress.discovered == 0 && self.has_files(source_id)? {
                self.mark_unseen_unavailable(&scope, generation)?;
                report
                    .diagnostics
                    .push(Failure::new(Kind::NoAudioFound).about(source.path.display()));
            } else {
                report.removed = self.purge(&scope, generation)?;
            }
        }
        progress(&report.progress);
        Ok(report)
    }
}

/// Existing directories under `root` covering `dirs`, without nested duplicates.
fn resolve_scope(root: &Path, dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut resolved = BTreeSet::new();
    for dir in dirs {
        if !dir.starts_with(root) {
            continue;
        }
        let mut dir = dir.as_path();
        // Files below a non-Unicode directory cannot be stored either.
        while dir != root && (!dir.is_dir() || dir.to_str().is_none()) {
            dir = dir.parent().unwrap_or(root);
        }
        resolved.insert(dir.to_path_buf());
    }
    if resolved.is_empty() || resolved.contains(root) {
        return vec![root.to_path_buf()];
    }
    let mut kept: Vec<PathBuf> = Vec::new();
    // Sorted order places ancestors before their descendants.
    for dir in resolved {
        if !kept.iter().any(|ancestor| dir.starts_with(ancestor)) {
            kept.push(dir);
        }
    }
    kept
}
