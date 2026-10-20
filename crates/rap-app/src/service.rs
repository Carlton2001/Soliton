//! Fixed background workers own all filesystem, SQLite and filtering work.
use crate::catalog::{self, Filter};
use crossbeam_channel::{Receiver, Sender, bounded};
use rap_core::{config::Settings, model::LibrarySnapshot};
use rap_library::{Cancellation, Library, ScanProgress};
use rap_platform::AppPaths;
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

pub enum Work {
    Scan(Vec<PathBuf>, Cancellation),
    Save(Settings),
    RemoveSource(i64),
}
pub struct View {
    pub snapshot: Arc<LibrarySnapshot>,
    pub visible: Vec<usize>,
    pub filter: Filter,
    pub facets: catalog::Facets,
}
pub enum Event {
    Progress(ScanProgress),
    Finished(bool),
    Removed,
    Error(String),
}
pub struct Services {
    pub work: Sender<Work>,
    pub events: Receiver<Event>,
    pub query: Sender<Filter>,
    pub views: Receiver<anyhow::Result<View>>,
}

impl Services {
    pub fn start(paths: AppPaths) -> Self {
        let (work_tx, work_rx) = bounded(16);
        let (events_tx, events_rx) = bounded(32);
        let (query_tx, query_rx) = bounded::<Filter>(32);
        let (views_tx, views_rx) = bounded(2);
        let reader_paths = paths.clone();
        std::thread::spawn(move || {
            let run = || -> anyhow::Result<()> {
                let mut library = Library::open(&paths.root)?;
                while let Ok(work) = work_rx.recv() {
                    match work {
                        Work::Save(settings) => {
                            if let Err(error) = settings.save(&paths.settings) {
                                let _ = events_tx.send(Event::Error(format!("{error:#}")));
                            }
                        }
                        Work::RemoveSource(id) => {
                            if let Err(error) = library.remove_source(id) {
                                let _ = events_tx.send(Event::Error(format!("{error:#}")));
                            }
                            let _ = events_tx.send(Event::Removed);
                        }
                        Work::Scan(paths, cancellation) => {
                            let ids = if paths.is_empty() {
                                library.sources()?.iter().map(|s| s.id).collect()
                            } else {
                                let mut ids = Vec::new();
                                for path in paths {
                                    match library.add_source(&path) {
                                        Ok(id) => ids.push(id),
                                        Err(error) => {
                                            let _ = events_tx.send(Event::Error(format!(
                                                "{}: {error:#}",
                                                path.display()
                                            )));
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
                                match library.scan(id, &cancellation, |progress| {
                                    if last.elapsed() > Duration::from_millis(100) {
                                        let _ =
                                            events_tx.try_send(Event::Progress(progress.clone()));
                                        last = Instant::now();
                                    }
                                }) {
                                    Ok(report) => {
                                        let _ = events_tx.send(Event::Progress(report.progress));
                                        for detail in report.diagnostics {
                                            let _ = events_tx.send(Event::Error(detail));
                                        }
                                    }
                                    Err(error) => {
                                        let _ = events_tx.send(Event::Error(format!("{error:#}")));
                                    }
                                }
                            }
                            let _ = events_tx.send(Event::Finished(cancellation.is_cancelled()));
                        }
                    }
                }
                Ok(())
            };
            if let Err(error) = run() {
                let _ = events_tx.send(Event::Error(format!("{error:#}")));
                let _ = events_tx.send(Event::Finished(false));
            }
        });
        // WAL reader stays usable while indexing commits batches. Poll only while UI
        // requests refreshes; coalesce typing to the newest filter before querying.
        std::thread::spawn(move || {
            while let Ok(next) = query_rx.recv() {
                let filter = query_rx.try_iter().last().unwrap_or(next);
                let result = Library::open(&reader_paths.root).and_then(|library| {
                    let snapshot = Arc::new(library.snapshot()?);
                    let visible = catalog::matches(&snapshot, &filter);
                    let facets = catalog::Facets::collect(&snapshot);
                    Ok(View {
                        snapshot,
                        visible,
                        filter,
                        facets,
                    })
                });
                if views_tx.send(result).is_err() {
                    break;
                }
            }
        });
        Self {
            work: work_tx,
            events: events_rx,
            query: query_tx,
            views: views_rx,
        }
    }
}
