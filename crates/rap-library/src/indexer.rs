use crate::{
    Library,
    discovery::{self, Candidate, Discovered},
    metadata,
};
use anyhow::{Result, bail};
use crossbeam_channel::bounded;
use std::{
    path::Path,
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
    pub complete: bool,
    /// Bounded diagnostic sample; total errors are counted separately.
    pub diagnostics: Vec<String>,
}
impl ScanReport {
    fn error(&mut self, text: String) {
        self.progress.errors += 1;
        if self.diagnostics.len() < 32 {
            self.diagnostics.push(text);
        }
    }
}

impl Library {
    /// Synchronous service API. Invoke from a background worker, never GPUI.
    pub fn scan(
        &mut self,
        source_id: i64,
        cancel: &Cancellation,
        mut progress: impl FnMut(&ScanProgress),
    ) -> Result<ScanReport> {
        let source = self
            .sources()?
            .into_iter()
            .find(|s| s.id == source_id)
            .ok_or_else(|| anyhow::anyhow!("Unknown source"))?;
        if let Err(error) = std::fs::read_dir(&source.path) {
            self.connection
                .execute("UPDATE sources SET available=0 WHERE id=?1", [source_id])?;
            bail!("{}: {error}", source.path.display());
        }
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
        std::thread::scope(|scope| -> Result<()> {
            let (found_tx, found_rx) = bounded(64);
            let (jobs_tx, jobs_rx) = bounded::<Candidate>(16);
            let (results_tx, results_rx) = bounded(16);
            let scan_cancel = cancel.clone();
            scope.spawn(move || {
                if let Err(error) = discovery::walk(&root, &scan_cancel, &found_tx) {
                    discovery::send(
                        &found_tx,
                        Discovered::Error(format!("{error:#}")),
                        &scan_cancel,
                    );
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
                            Ok(Discovered::File(file)) => {
                                report.progress.discovered += 1;
                                if self.fingerprint(source_id, &file.path)?.is_some_and(|(size, modified)| size == file.size && modified == file.modified) {
                                    self.seen(source_id, &file.path, generation)?;
                                    report.progress.unchanged += 1;
                                } else { pending = Some(file); }
                            }
                            Ok(Discovered::Error(error)) => report.error(error),
                            Err(_) => { found = None; jobs.take(); }
                        },
                        recv(results_rx) -> message => match message {
                            Ok((file, value)) => {
                                self.seen(source_id, &file.path, generation)?;
                                match value {
                                    Ok(value) => {
                                        self.store(source_id, &file.path, file.size, &file.modified, generation, &value.metadata, value.cover.as_deref())?;
                                        report.progress.indexed += 1;
                                        for warning in value.warnings { report.error(warning); }
                                    }
                                    Err(error) => report.error(format!("{}: {error:#}", file.path.display())),
                                }
                                batched += 1;
                            }
                            Err(_) => break,
                        },
                        default(std::time::Duration::from_millis(30)) => {},
                    }
                    if batched >= 64 {
                        self.connection.execute_batch("COMMIT; BEGIN IMMEDIATE")?;
                        batched = 0;
                    }
                    progress(&report.progress);
                }
                Ok(())
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
        report.complete = !report.cancelled && report.progress.errors == 0;
        // A partial traversal can never establish disappearance. Never delete rows.
        if report.complete && Path::new(&source.path).is_dir() {
            self.connection.execute(
                "UPDATE files SET available=0 WHERE source_id=?1 AND seen<>?2",
                rusqlite::params![source_id, generation],
            )?;
        }
        progress(&report.progress);
        Ok(report)
    }
}
