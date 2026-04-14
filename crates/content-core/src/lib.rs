//! Content management for Word Mobile.
//!
//! This crate provides vocabulary, wordbook, and content import services.
//! It handles content validation and processing but delegates persistence
//! to the caller.

pub mod snapshot;
pub mod vocabulary_import;
pub mod vocabulary_status;
pub mod wordbook_derive;

pub use snapshot::{check_bundled_snapshot, SnapshotResult};
