//! Library persistence and indexing, independent of the UI.
mod database;
mod discovery;
mod indexer;
mod metadata;

pub use database::Library;
pub use indexer::{Cancellation, ScanProgress, ScanReport};
