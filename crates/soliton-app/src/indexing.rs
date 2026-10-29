//! The single library writer: user scans, folder notifications, availability probes
//! and periodic incremental scans, all serialised on one worker thread. Playlist
//! edits pass between scan batches; other work waits for the scan.
use crate::service::{Event, Work};
use crossbeam_channel::{Receiver, Sender, bounded};
use soliton_core::{
    failure::{Failure, Kind},
    playlist::PlaylistChange,
};
use soliton_library::{
    Cancellation, Coalescer, Interlude, Library, Prober, ScanProgress, SourceWatcher, WatchEvent,
};
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Safety net for shares whose notifications are unreliable.
const PERIODIC_SCAN: Duration = Duration::from_secs(30 * 60);
/// One directory read per source: cheap enough to notice an unmounted disk soon.
const PROBE_INTERVAL: Duration = Duration::from_secs(5);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// Offline sources are probed more often, so that a remounted volume returns
/// quickly; a share still slow to answer is simply tried again later.
const RETURN_PROBE_INTERVAL: Duration = Duration::from_secs(5);
const RETURN_PROBE_TIMEOUT: Duration = Duration::from_secs(1);
/// A share that stops answering is offline only once silent this long: disks
/// of a sleeping NAS may take that long to spin up, and a definite refusal
/// still counts at once.
const SILENCE_LIMIT: Duration = Duration::from_secs(30);
/// A source not yet watched must answer this fast before its watch is registered.
const WATCH_PROBE_TIMEOUT: Duration = Duration::from_secs(1);
/// A burst of notifications is handled once quiet, or after the longest delay.
const QUIET: Duration = Duration::from_secs(2);
const LONGEST: Duration = Duration::from_secs(15);

struct Worker {
    library: Library,
    work: Receiver<Work>,
    /// Work received during a scan that could not pass between its batches.
    deferred: VecDeque<Work>,
    events: Sender<Event>,
    watcher: Option<SourceWatcher>,
    changes: Coalescer,
    prober: Prober,
    silence: Silence,
    /// Background work stops when the application closes.
    background: Cancellation,
    next_probe: Instant,
    next_return_probe: Instant,
    next_periodic: Instant,
}

pub fn run(root: &Path, work: Receiver<Work>, events: Sender<Event>, background: Cancellation) {
    let library = match Library::open(root) {
        Ok(library) => library,
        Err(error) => {
            let _ = events.send(Event::Error(Failure::of(&error)));
            let _ = events.send(Event::Finished(false));
            return;
        }
    };
    let (notify_tx, notify_rx) = bounded(256);
    match library.playlists() {
        Ok(lists) => {
            let _ = events.send(Event::Playlists(lists));
        }
        Err(error) => {
            let _ = events.send(Event::Error(Failure::of(&error)));
        }
    }
    let watcher = match SourceWatcher::new(notify_tx) {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            // Scans still keep the library current without notifications.
            let _ = events.send(Event::Error(Failure::of(&error)));
            None
        }
    };
    let now = Instant::now();
    let mut worker = Worker {
        prober: library.prober(),
        library,
        work,
        deferred: VecDeque::new(),
        events,
        watcher,
        changes: Coalescer::new(QUIET, LONGEST),
        silence: Silence::default(),
        background,
        next_probe: now + PROBE_INTERVAL,
        next_return_probe: now + RETURN_PROBE_INTERVAL,
        next_periodic: now + PERIODIC_SCAN,
    };
    loop {
        if worker.background.is_cancelled() {
            break;
        }
        if let Some(message) = worker.deferred.pop_front() {
            worker.handle(message);
            worker.timers();
            continue;
        }
        let deadline = [
            worker.next_probe,
            worker.next_return_probe,
            worker.next_periodic,
        ]
        .into_iter()
        .chain(worker.changes.deadline())
        .min()
        .unwrap_or_else(Instant::now);
        let timeout = deadline.saturating_duration_since(Instant::now());
        crossbeam_channel::select! {
            recv(worker.work) -> message => match message {
                Ok(message) => worker.handle(message),
                Err(_) => break,
            },
            recv(notify_rx) -> event => {
                if let Ok(event) = event {
                    worker.changes.push(event, Instant::now());
                }
                for event in notify_rx.try_iter() {
                    worker.changes.push(event, Instant::now());
                }
            },
            default(timeout) => {},
        }
        if worker
            .watcher
            .as_ref()
            .is_some_and(SourceWatcher::overflowed)
        {
            worker.changes.push(WatchEvent::Overflow, Instant::now());
        }
        worker.timers();
    }
}

impl Worker {
    fn send(&self, event: Event) {
        let _ = self.events.send(event);
    }

    fn error(&self, error: &anyhow::Error) {
        self.send(Event::Error(Failure::of(error)));
    }

    fn handle(&mut self, work: Work) {
        match work {
            Work::Playlist(change) => change_playlist(&mut self.library, &self.events, change),
            Work::RemoveSource(id) => {
                if let Err(error) = self.library.remove_source(id) {
                    self.error(&error);
                }
                self.sync_watcher();
                self.send(Event::Removed);
                match self.library.playlists() {
                    Ok(lists) => self.send(Event::Playlists(lists)),
                    Err(error) => self.error(&error),
                }
            }
            Work::Scan(paths, cancellation) => self.user_scan(paths, &cancellation),
        }
    }

    /// Adds `paths` as sources, or rescans every source when empty, with progress.
    fn user_scan(&mut self, paths: Vec<PathBuf>, cancellation: &Cancellation) {
        let everything = paths.is_empty();
        let ids = if everything {
            self.sources().iter().map(|s| s.id).collect()
        } else {
            let mut ids = Vec::new();
            for path in paths {
                match self.library.add_source(&path) {
                    Ok(id) => ids.push(id),
                    Err(error) => {
                        let failure = Failure::of(&error);
                        // Library failures already name the folder.
                        let failure = if failure.detail.is_empty() {
                            failure.about(path.display())
                        } else {
                            failure
                        };
                        self.send(Event::Error(failure));
                    }
                }
            }
            ids
        };
        for id in ids {
            if cancellation.is_cancelled() {
                break;
            }
            let mut last = Instant::now() - Duration::from_secs(1);
            let events = self.events.clone();
            let result = self.library.scan_scope(
                id,
                &[],
                cancellation,
                |progress| {
                    if last.elapsed() > Duration::from_millis(100) {
                        let _ = events.try_send(Event::Progress(progress.clone()));
                        last = Instant::now();
                    }
                },
                &mut Passage {
                    work: &self.work,
                    deferred: &mut self.deferred,
                    events: &self.events,
                },
            );
            match result {
                Ok(report) => {
                    self.send(Event::Progress(report.progress));
                    for detail in report.diagnostics {
                        self.send(Event::Warning(detail));
                    }
                }
                Err(error) => self.error(&error),
            }
        }
        if everything {
            self.next_periodic = Instant::now() + PERIODIC_SCAN;
            self.prune_covers();
        }
        self.sync_watcher();
        self.send(Event::Finished(cancellation.is_cancelled()));
    }

    /// Scans without progress display; the interface only refreshes its view.
    fn quiet_scan(&mut self, id: i64, dirs: &[PathBuf]) {
        let was_available = self.sources().iter().any(|s| s.id == id && s.available);
        let mut last = Instant::now();
        let mut shown = 0;
        let mut announced = was_available;
        let events = self.events.clone();
        let result = self.library.scan_scope(
            id,
            dirs,
            &self.background,
            |progress: &ScanProgress| {
                // A returning source is shown available before its scan ends.
                if !announced {
                    announced = true;
                    let _ = events.try_send(Event::Changed);
                }
                if progress.indexed > shown && last.elapsed() > Duration::from_secs(1) {
                    shown = progress.indexed;
                    last = Instant::now();
                    let _ = events.try_send(Event::Changed);
                }
            },
            &mut Passage {
                work: &self.work,
                deferred: &mut self.deferred,
                events: &self.events,
            },
        );
        let report = match result {
            Ok(report) => report,
            Err(error) => {
                match self.sources().into_iter().find(|s| s.id == id) {
                    Some(source) if was_available && !source.available => {
                        self.send(Event::SourceUnavailable(source.path));
                    }
                    // A slow source is left to the availability probes.
                    _ if Failure::of(&error).kind == Kind::SourceNotResponding => {}
                    Some(_) if was_available => self.error(&error),
                    _ => {}
                }
                self.send(Event::Changed);
                return;
            }
        };
        for detail in report.diagnostics.iter().cloned() {
            self.send(Event::Warning(detail));
        }
        if report.progress.indexed > 0 || report.removed > 0 || !was_available {
            self.send(Event::Changed);
        }
    }

    fn timers(&mut self) {
        let now = Instant::now();
        if let Some(batch) = self.changes.take(now) {
            match batch {
                Some(paths) => {
                    let sources = self.sources();
                    for (id, dirs) in soliton_library::scopes(&sources, &paths) {
                        if sources.iter().any(|s| s.id == id && s.available) {
                            self.quiet_scan(id, &dirs);
                        }
                    }
                }
                None => self.periodic(),
            }
        }
        if Instant::now() >= self.next_periodic {
            self.periodic();
        }
        if Instant::now() >= self.next_probe {
            self.probe(false);
            self.next_probe = Instant::now() + PROBE_INTERVAL;
        } else if Instant::now() >= self.next_return_probe {
            self.probe(true);
        }
        if Instant::now() >= self.next_return_probe {
            self.next_return_probe = Instant::now() + RETURN_PROBE_INTERVAL;
        }
    }

    /// Incremental scan of every reachable source, then cache cleanup.
    fn periodic(&mut self) {
        for source in self.sources() {
            if self.background.is_cancelled() {
                return;
            }
            if source.available {
                self.quiet_scan(source.id, &[]);
            }
        }
        self.prune_covers();
        self.sync_watcher();
        self.next_periodic = Instant::now() + PERIODIC_SCAN;
    }

    /// Detects sources going offline, and rescans those coming back; only the
    /// offline ones with `returning`.
    fn probe(&mut self, returning: bool) {
        let mut changed = false;
        for source in self.sources() {
            if self.background.is_cancelled() {
                return;
            }
            if returning && source.available {
                continue;
            }
            let timeout = if returning {
                RETURN_PROBE_TIMEOUT
            } else {
                PROBE_TIMEOUT
            };
            let answer = self.prober.reachable(&source.path, timeout);
            if !source.available {
                self.silence.forget(source.id);
                if answer == Some(true) {
                    changed = true;
                    self.quiet_scan(source.id, &[]);
                }
            } else if self
                .silence
                .unreachable(source.id, answer, Instant::now(), SILENCE_LIMIT)
            {
                match self.library.set_source_available(source.id, false) {
                    Ok(true) => {
                        changed = true;
                        self.send(Event::SourceUnavailable(source.path));
                    }
                    Ok(false) => {}
                    Err(error) => self.error(&error),
                }
            }
        }
        if changed {
            self.sync_watcher();
            self.send(Event::Changed);
        }
    }

    /// Thumbnails are kept while any track references them, offline sources included.
    /// Safe between scans only: a running scan may not have stored new references yet.
    fn prune_covers(&self) {
        if let Err(error) = self.library.prune_covers() {
            self.error(&error);
        }
    }

    fn sync_watcher(&mut self) {
        let sources = self.sources();
        let prober = &self.prober;
        if let Some(watcher) = &mut self.watcher {
            let reachable = |path: &Path| prober.reachable(path, WATCH_PROBE_TIMEOUT) == Some(true);
            for error in watcher.sync(&sources, reachable) {
                tracing::warn!("Folder notifications unavailable: {error}");
            }
        }
    }

    fn sources(&self) -> Vec<soliton_core::model::Source> {
        self.library.sources().unwrap_or_else(|error| {
            self.error(&error);
            Vec::new()
        })
    }
}

/// When each available source started leaving probes unanswered.
#[derive(Default)]
struct Silence(HashMap<i64, Instant>);

impl Silence {
    /// Whether an available source counts as offline after `answer`: at once
    /// when refused, after `limit` of unanswered probes otherwise.
    fn unreachable(
        &mut self,
        id: i64,
        answer: Option<bool>,
        now: Instant,
        limit: Duration,
    ) -> bool {
        match answer {
            Some(reachable) => {
                self.0.remove(&id);
                !reachable
            }
            None => now.duration_since(*self.0.entry(id).or_insert(now)) >= limit,
        }
    }

    /// An offline source starts afresh once back.
    fn forget(&mut self, id: i64) {
        self.0.remove(&id);
    }
}

fn change_playlist(library: &mut Library, events: &Sender<Event>, change: PlaylistChange) {
    if let Err(error) = library.change_playlist(change) {
        let _ = events.send(Event::Error(Failure::of(&error)));
    }
    let event = match library.playlists() {
        Ok(lists) => Event::Playlists(lists),
        Err(error) => Event::Error(Failure::of(&error)),
    };
    let _ = events.send(event);
}

/// Lets playlist edits through a running scan; scans and source changes wait.
struct Passage<'a> {
    work: &'a Receiver<Work>,
    deferred: &'a mut VecDeque<Work>,
    events: &'a Sender<Event>,
}

impl Interlude for Passage<'_> {
    fn waiting(&self) -> bool {
        !self.work.is_empty()
    }
    fn run(&mut self, library: &mut Library) {
        for message in self.work.try_iter() {
            match message {
                Work::Playlist(change) => change_playlist(library, self.events, change),
                other => self.deferred.push_back(other),
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/indexing.rs"]
mod tests;
