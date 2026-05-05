//! Storage layer for Word Mobile.
//!
//! This crate provides database persistence and domain models.
//! It is platform-agnostic and can be used by any platform shell.

pub mod models;
pub mod persistence;

pub use models::*;
pub use persistence::{
    entry_repo, plan_repo, study_repo, word_hint_repo, wordbook_repo, StorageError,
};
pub use rusqlite::Connection;
