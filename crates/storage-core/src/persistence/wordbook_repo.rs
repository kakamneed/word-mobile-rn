use rusqlite::{Connection, Row};

use crate::models::Wordbook;
use crate::StorageError;

/// Insert a wordbook.
pub fn insert_wordbook(
    conn: &Connection,
    code: &str,
    name: &str,
    category: &str,
    description: Option<&str>,
    source_book_id: &str,
    source_version_id: i64,
    total_entries: i64,
) -> Result<i64, StorageError> {
    conn.execute(
        "INSERT INTO wordbooks (code, name, category, description, source_book_id, source_version_id, total_entries, is_active)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
        rusqlite::params![code, name, category, description, source_book_id, source_version_id, total_entries],
    )
    .map_err(|e| StorageError::Database(format!("Failed to insert wordbook: {e}")))?;

    Ok(conn.last_insert_rowid())
}

/// Get all wordbooks.
pub fn get_all_wordbooks(conn: &Connection) -> Result<Vec<Wordbook>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, code, name, category, description, source_book_id, source_version_id, total_entries, is_active
             FROM wordbooks
             ORDER BY code",
        )
        .map_err(|e| StorageError::Database(format!("Failed to prepare: {e}")))?;

    let rows = stmt
        .query_map([], |row| row_to_wordbook(row))
        .map_err(|e| StorageError::Database(format!("Failed to query: {e}")))?;

    let mut wordbooks = Vec::new();
    for row in rows {
        wordbooks
            .push(row.map_err(|e| StorageError::Database(format!("Failed to read row: {e}")))?);
    }

    Ok(wordbooks)
}

/// Get active wordbooks.
pub fn get_active_wordbooks(conn: &Connection) -> Result<Vec<Wordbook>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, code, name, category, description, source_book_id, source_version_id, total_entries, is_active
             FROM wordbooks
             WHERE is_active = 1
             ORDER BY code",
        )
        .map_err(|e| StorageError::Database(format!("Failed to prepare: {e}")))?;

    let rows = stmt
        .query_map([], |row| row_to_wordbook(row))
        .map_err(|e| StorageError::Database(format!("Failed to query: {e}")))?;

    let mut wordbooks = Vec::new();
    for row in rows {
        wordbooks
            .push(row.map_err(|e| StorageError::Database(format!("Failed to read row: {e}")))?);
    }

    Ok(wordbooks)
}

/// Deactivate all wordbooks.
pub fn deactivate_all_wordbooks(conn: &Connection) -> Result<(), StorageError> {
    conn.execute("UPDATE wordbooks SET is_active = 0", [])
        .map_err(|e| StorageError::Database(format!("Failed to deactivate: {e}")))?;
    Ok(())
}

/// Activate wordbooks by version.
pub fn activate_wordbooks_by_version(
    conn: &Connection,
    version_id: i64,
) -> Result<(), StorageError> {
    conn.execute(
        "UPDATE wordbooks SET is_active = 1 WHERE source_version_id = ?1",
        rusqlite::params![version_id],
    )
    .map_err(|e| StorageError::Database(format!("Failed to activate: {e}")))?;
    Ok(())
}

fn row_to_wordbook(row: &Row) -> Result<Wordbook, rusqlite::Error> {
    let is_active_raw: i64 = row.get(8)?;
    Ok(Wordbook {
        id: Some(row.get(0)?),
        code: row.get(1)?,
        name: row.get(2)?,
        category: row.get(3)?,
        description: row.get(4)?,
        source_book_id: row.get(5)?,
        source_version_id: Some(row.get(6)?),
        total_entries: row.get(7)?,
        is_active: is_active_raw != 0,
    })
}
