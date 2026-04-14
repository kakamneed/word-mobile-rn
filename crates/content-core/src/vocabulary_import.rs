//! Placeholder for vocabulary import functionality.
//!
//! Phase 1: Basic structure only.

use std::path::Path;

/// Import vocabulary from a snapshot file.
pub fn import_vocabulary_snapshot(_path: &Path) -> Result<ImportResult, ImportError> {
    // Phase 1 placeholder
    Ok(ImportResult {
        entries_imported: 0,
        wordbooks_created: 0,
    })
}

/// Result of vocabulary import.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub entries_imported: usize,
    pub wordbooks_created: usize,
}

/// Import error.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Parse error: {0}")]
    Parse(String),
}
