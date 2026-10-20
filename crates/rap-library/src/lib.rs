//! Library persistence and indexing, independent of the UI.
mod availability;
mod catalog;
mod covers;
mod database;
mod discovery;
mod grouping;
mod indexer;
mod metadata;
mod schema;
mod standalone;
mod watch;

pub use availability::Prober;
pub use database::Library;
pub use indexer::{Cancellation, ScanProgress, ScanReport};
pub use standalone::{LooseFiles, read_loose_files};
pub use watch::{Coalescer, SourceWatcher, WatchEvent, scopes};
