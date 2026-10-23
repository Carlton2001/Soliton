//! The single library writer: user scans, folder notifications, availability probes
//! and periodic incremental scans, all serialised on one worker thread.
use crate::service::{Event, Work};
use crossbeam_channel::{Receiver, Sender, bounded};
use rap_library::{
    Cancellation, Coalescer, Library, Prober, ScanProgress, SourceWatcher, WatchEvent,
};
use rap_platform::AppPaths;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

/// Safety net for shares whose notifications are unreliable.
const PERIODIC_SCAN: Duration = Duration::from_secs(30 * 60);
const PROBE_INTERVAL: Duration = Duration::from_secs(60);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// A burst of notifications is handled once quiet, or after the longest delay.
const QUIET: Duration = Duration::from_secs(2);
const LONGEST: Duration = Duration::from_secs(15);

struct Worker {
    paths: AppPaths,
    library: Library,
    events: Sender<Event>,
    watcher: Option<SourceWatcher>,
    changes: Coalescer,
    prober: Prober,
    /// Background work stops when the application closes.
    background: Cancellation,
    next_probe: Instant,
    next_periodic: Instant,
}

pub fn run(paths: AppPaths, work: Receiver<Work>, events: Sender<Event>, background: Cancellation) {
    let library = match Library::open(&paths.root) {
        Ok(library) => library,
        Err(error) => {
            let _ = events.send(Event::Error(format!("{error:#}")));
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
            let _ = events.send(Event::Error(format!("{error:#}")));
        }
    }
    let watcher = match SourceWatcher::new(notify_tx) {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            // Scans still keep the library current without notifications.
            let _ = events.send(Event::Error(format!("{error:#}")));
            None
        }
    };
    let now = Instant::now();
    let mut worker = Worker {
        paths,
        library,
        events,
        watcher,
        changes: Coalescer::new(QUIET, LONGEST),
        prober: Prober::default(),
        background,
        next_probe: now + PROBE_INTERVAL,
        next_periodic: now + PERIODIC_SCAN,
    };
    loop {
        if worker.background.is_cancelled() {
            break;
        }
        let deadline = [worker.next_probe, worker.next_periodic]
            .into_iter()
            .chain(worker.changes.deadline())
            .min()
            .unwrap_or_else(Instant::now);
        let timeout = deadline.saturating_duration_since(Instant::now());
        crossbeam_channel::select! {
            recv(work) -> message => match message {
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

    fn error(&self, error: impl std::fmt::Display) {
        self.send(Event::Error(error.to_string()));
    }

    fn handle(&mut self, work: Work) {
        match work {
            Work::Playlist(change) => {
                if let Err(error) = self.library.change_playlist(change) {
                    self.error(format_args!("{error:#}"));
                }
                match self.library.playlists() {
                    Ok(lists) => self.send(Event::Playlists(lists)),
                    Err(error) => self.error(format_args!("{error:#}")),
                }
            }
            Work::Save(settings) => {
                if let Err(error) = settings.save(&self.paths.settings) {
                    self.error(format_args!("{error:#}"));
                }
            }
            Work::SaveAndFlush(settings, reply) => {
                let _ = reply.try_send(settings.save(&self.paths.settings));
            }
            Work::RemoveSource(id) => {
                if let Err(error) = self.library.remove_source(id) {
                    self.error(format_args!("{error:#}"));
                }
                self.sync_watcher();
                self.send(Event::Removed);
                match self.library.playlists() {
                    Ok(lists) => self.send(Event::Playlists(lists)),
                    Err(error) => self.error(format_args!("{error:#}")),
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
                    Err(error) => self.error(format_args!("{}: {error:#}", path.display())),
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
            let result = self.library.scan(id, cancellation, |progress| {
                if last.elapsed() > Duration::from_millis(100) {
                    let _ = events.try_send(Event::Progress(progress.clone()));
                    last = Instant::now();
                }
            });
            match result {
                Ok(report) => {
                    self.send(Event::Progress(report.progress));
                    for detail in report.diagnostics {
                        self.send(Event::Warning(detail));
                    }
                }
                Err(error) => self.error(format_args!("{error:#}")),
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
        let events = self.events.clone();
        let result =
            self.library
                .scan_scope(id, dirs, &self.background, |progress: &ScanProgress| {
                    if progress.indexed > shown && last.elapsed() > Duration::from_secs(1) {
                        shown = progress.indexed;
                        last = Instant::now();
                        let _ = events.try_send(Event::Changed);
                    }
                });
        let report = match result {
            Ok(report) => report,
            Err(error) => {
                match self.sources().into_iter().find(|s| s.id == id) {
                    Some(source) if was_available && !source.available => {
                        self.send(Event::SourceUnavailable(source.path));
                    }
                    Some(_) if was_available => self.error(format_args!("{error:#}")),
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
                    for (id, dirs) in rap_library::scopes(&sources, &paths) {
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
            self.probe();
            self.next_probe = Instant::now() + PROBE_INTERVAL;
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

    /// Detects sources going offline, and rescans those coming back.
    fn probe(&mut self) {
        let mut changed = false;
        for source in self.sources() {
            if self.background.is_cancelled() {
                return;
            }
            let reachable = self.prober.reachable(&source.path, PROBE_TIMEOUT) == Some(true);
            if source.available && !reachable {
                match self.library.set_source_available(source.id, false) {
                    Ok(true) => {
                        changed = true;
                        self.send(Event::SourceUnavailable(source.path));
                    }
                    Ok(false) => {}
                    Err(error) => self.error(format_args!("{error:#}")),
                }
            } else if !source.available && reachable {
                changed = true;
                self.quiet_scan(source.id, &[]);
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
            self.error(format_args!("{error:#}"));
        }
    }

    fn sync_watcher(&mut self) {
        let sources = self.sources();
        if let Some(watcher) = &mut self.watcher {
            for error in watcher.sync(&sources) {
                tracing::warn!("Folder notifications unavailable: {error}");
            }
        }
    }

    fn sources(&self) -> Vec<rap_core::model::Source> {
        self.library.sources().unwrap_or_else(|error| {
            self.error(format_args!("{error:#}"));
            Vec::new()
        })
    }
}
