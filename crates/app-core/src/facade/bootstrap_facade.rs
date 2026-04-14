//! Bootstrap facade for app initialization.

use std::path::Path;

use rusqlite::Connection;

use word_storage_core::models::BootstrapState;
use word_storage_core::persistence;

use crate::bootstrap::evaluate_bootstrap;
use crate::platform::{PlatformError, PlatformRuntime};

/// Errors that can occur during bootstrap.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("Platform error: {0}")]
    Platform(#[from] PlatformError),
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Bootstrap the application.
///
/// This is the main entry point for app initialization. It:
/// 1. Ensures directories exist
/// 2. Opens/creates the database
/// 3. Evaluates the full bootstrap state
///
/// # Example
///
/// ```rust,ignore
/// let runtime = MyPlatformRuntime::new();
/// let db_path = runtime.app_data_dir()?.join("word.db");
/// let state = bootstrap(&runtime, &db_path)?;
/// ```
pub fn bootstrap(
    runtime: &dyn PlatformRuntime,
    db_path: &Path,
) -> Result<BootstrapState, BootstrapError> {
    let conn = persistence::initialize_database(db_path)
        .map_err(|e| BootstrapError::Storage(e.to_string()))?;
    evaluate_bootstrap(runtime, &conn).map_err(|e| BootstrapError::Platform(e.into()))
}

/// Bootstrap with an existing connection (for testing).
pub fn bootstrap_with_connection(
    runtime: &dyn PlatformRuntime,
    conn: &Connection,
) -> Result<BootstrapState, BootstrapError> {
    evaluate_bootstrap(runtime, conn).map_err(|e| BootstrapError::Platform(e.into()))
}
