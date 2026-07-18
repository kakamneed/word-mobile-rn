//! Persistence layer for Word Mobile storage.
//!
//! Provides database connection management, schema migrations, and
//! repository modules for each domain area.

use std::path::Path;

pub mod entry_repo;
pub mod exercise_vocab_repo;
pub mod mastered_entry_repo;
pub mod plan_repo;
pub mod schema;
#[cfg(test)]
mod schema_tests;
pub mod study_repo;
pub mod sync_repo;
pub mod user_accepted_meaning_repo;
pub mod word_hint_repo;
pub mod wordbook_repo;
pub use rusqlite::Connection;

/// Errors that can occur during storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Connection error: {0}")]
    Connection(String),
    #[error("Schema error: {0}")]
    Schema(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

/// Initialize the database at the given path.
///
/// Creates the database file and parent directories if they don't exist,
/// then applies the schema migrations.
pub fn initialize_database(db_path: &Path) -> Result<Connection, StorageError> {
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| StorageError::Connection(format!("Failed to create directory: {e}")))?;
    }

    let conn = Connection::open(db_path)
        .map_err(|e| StorageError::Connection(format!("Failed to open database: {e}")))?;

    schema::apply_schema(&conn)?;

    Ok(conn)
}

/// Open an existing database without applying schema changes.
///
/// Use this for read-only operations or when schema is managed elsewhere.
pub fn open_database(db_path: &Path) -> Result<Connection, StorageError> {
    let conn = Connection::open(db_path)
        .map_err(|e| StorageError::Connection(format!("Failed to open database: {e}")))?;
    Ok(conn)
}

/// Check if onboarding has been completed.
pub fn is_onboarding_completed(conn: &Connection) -> Result<bool, StorageError> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_metadata WHERE key = 'onboarding_completed'",
            [],
            |row| row.get(0),
        )
        .ok();

    Ok(value.as_deref() == Some("true"))
}

/// Mark onboarding as completed.
pub fn mark_onboarding_completed(conn: &Connection) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO app_metadata (key, value) VALUES ('onboarding_completed', 'true')
         ON CONFLICT(key) DO UPDATE SET value = 'true', updated_at = datetime('now')",
        [],
    )
    .map_err(|e| StorageError::Database(format!("Failed to mark onboarding: {e}")))?;

    Ok(())
}
