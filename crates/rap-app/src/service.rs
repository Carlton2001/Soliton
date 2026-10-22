//! Fixed background workers own all filesystem, SQLite and filtering work.
use crate::catalog::{self, Catalog, Filter};
use crossbeam_channel::{Receiver, Sender, bounded};
use rap_core::{config::Settings, model::LibrarySnapshot};
use rap_library::{Cancellation, Library, ScanProgress};
use rap_platform::AppPaths;
use std::{path::PathBuf, sync::Arc};

pub enum Work {
    Scan(Vec<PathBuf>, Cancellation),
    Save(Settings),
    SaveAndFlush(Settings, Sender<anyhow::Result<()>>),
    RemoveSource(i64),
}
pub struct View {
    pub snapshot: Arc<LibrarySnapshot>,
    pub visible: Vec<usize>,
    pub filter: Filter,
    pub facets: Arc<catalog::Facets>,
    pub options: Arc<catalog::FilterOptions>,
}
pub enum Event {
    Progress(ScanProgress),
    Finished(bool),
    Removed,
    /// Background work changed the library; the view should be queried again.
    Changed,
    /// A source stopped answering; its entries are kept.
    SourceUnavailable(PathBuf),
    /// A file could not be read; the rest of the work went on.
    Warning(String),
    Error(String),
}
pub struct Services {
    pub work: Sender<Work>,
    pub events: Receiver<Event>,
    pub query: Sender<Filter>,
    pub views: Receiver<anyhow::Result<View>>,
    background: Cancellation,
}

impl Services {
    pub fn start(paths: AppPaths) -> Self {
        let (work_tx, work_rx) = bounded(16);
        let (events_tx, events_rx) = bounded(32);
        let (query_tx, query_rx) = bounded::<Filter>(32);
        let (views_tx, views_rx) = bounded(2);
        let background = Cancellation::default();
        let reader_root = paths.root.clone();
        let worker_background = background.clone();
        std::thread::Builder::new()
            .name("rap-library".into())
            .spawn(move || crate::indexing::run(paths, work_rx, events_tx, worker_background))
            .expect("Cannot start the library worker");
        // The WAL reader stays usable while indexing commits batches. Typing is
        // coalesced to the newest filter; the catalogue is rebuilt only after commits.
        std::thread::Builder::new()
            .name("rap-query".into())
            .spawn(move || {
                let mut library = None;
                let mut cached: Option<(i64, Catalog)> = None;
                while let Ok(next) = query_rx.recv() {
                    let filter = query_rx.try_iter().last().unwrap_or(next);
                    let result = (|| {
                        if library.is_none() {
                            library = Some(Library::open_reader(&reader_root)?);
                        }
                        let library = library.as_ref().unwrap();
                        let version = library.data_version()?;
                        if cached.as_ref().is_none_or(|(known, _)| *known != version) {
                            cached = Some((version, Catalog::new(library.snapshot()?)));
                        }
                        let catalog = &cached.as_ref().unwrap().1;
                        Ok(View {
                            snapshot: catalog.snapshot.clone(),
                            visible: catalog.matches(&filter),
                            options: Arc::new(catalog.options(&filter)),
                            filter,
                            facets: catalog.facets.clone(),
                        })
                    })();
                    if result.is_err() {
                        library = None;
                    }
                    if views_tx.send(result).is_err() {
                        break;
                    }
                }
            })
            .expect("Cannot start the query worker");
        Self {
            work: work_tx,
            events: events_rx,
            query: query_tx,
            views: views_rx,
            background,
        }
    }
}

impl Drop for Services {
    fn drop(&mut self) {
        self.background.cancel();
    }
}
