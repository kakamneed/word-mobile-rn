//! Repository for words the learner has marked as mastered.

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension};

use crate::StorageError;

pub fn mark_mastered_by_source_id(
    conn: &Connection,
    entry_source_id: &str,
    reason: &str,
) -> Result<Option<i64>, StorageError> {
    let entry_id = resolve_entry_id(conn, entry_source_id)?;
    conn.execute(
        "INSERT INTO mastered_entries (entry_id, source_entry_id, reason, mastered_at)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(source_entry_id) DO UPDATE SET
            entry_id = excluded.entry_id,
            reason = excluded.reason,
            mastered_at = excluded.mastered_at",
        rusqlite::params![entry_id, entry_source_id, reason],
    )
    .map_err(|e| StorageError::Database(format!("Failed to mark mastered entry: {e}")))?;
    Ok(entry_id)
}

pub fn mastered_entry_ids(conn: &Connection) -> Result<BTreeSet<i64>, StorageError> {
    let mut stmt = conn
        .prepare("SELECT entry_id FROM mastered_entries WHERE entry_id IS NOT NULL")
        .map_err(|e| StorageError::Database(format!("Failed to prepare mastered query: {e}")))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|e| StorageError::Database(format!("Failed to query mastered entries: {e}")))?;
    let mut out = BTreeSet::new();
    for row in rows {
        out.insert(row.map_err(|e| {
            StorageError::Database(format!("Failed to read mastered entry id: {e}"))
        })?);
    }
    Ok(out)
}

pub fn is_mastered_source_id(
    conn: &Connection,
    entry_source_id: &str,
) -> Result<bool, StorageError> {
    let entry_id = resolve_entry_id(conn, entry_source_id)?;
    let exists = if let Some(entry_id) = entry_id {
        conn.query_row(
            "SELECT 1 FROM mastered_entries
             WHERE entry_id = ?1 OR source_entry_id = ?2
             LIMIT 1",
            rusqlite::params![entry_id, entry_source_id],
            |_| Ok(()),
        )
        .optional()
    } else {
        conn.query_row(
            "SELECT 1 FROM mastered_entries WHERE source_entry_id = ?1 LIMIT 1",
            [entry_source_id],
            |_| Ok(()),
        )
        .optional()
    }
    .map_err(|e| StorageError::Database(format!("Failed to query mastered entry: {e}")))?;
    Ok(exists.is_some())
}

fn resolve_entry_id(conn: &Connection, entry_source_id: &str) -> Result<Option<i64>, StorageError> {
    if let Ok(entry_id) = entry_source_id.parse::<i64>() {
        let exists = conn
            .query_row(
                "SELECT 1 FROM entries WHERE id = ?1 LIMIT 1",
                [entry_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| StorageError::Database(format!("Failed to verify entry id: {e}")))?;
        return Ok(exists.map(|_| entry_id));
    }
    conn.query_row(
        "SELECT id FROM entries WHERE source_entry_key = ?1 ORDER BY id ASC LIMIT 1",
        [entry_source_id],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(|e| StorageError::Database(format!("Failed to resolve entry id: {e}")))
}

#[cfg(test)]
mod tests {
    use super::{is_mastered_source_id, mark_mastered_by_source_id, mastered_entry_ids};

    #[test]
    fn mark_mastered_is_idempotent_and_queryable_by_source_id() {
        let conn = rusqlite::Connection::open_in_memory().expect("open database");
        crate::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'mastered-test', 'ready')",
            [],
        )
        .expect("insert source");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma)
             VALUES (10, 1, 'source-key-10', 'abandon', 'abandon')",
            [],
        )
        .expect("insert entry");

        assert_eq!(
            mark_mastered_by_source_id(&conn, "source-key-10", "mastered").expect("mark"),
            Some(10)
        );
        let _ = mark_mastered_by_source_id(&conn, "source-key-10", "mastered").expect("mark again");
        assert!(is_mastered_source_id(&conn, "source-key-10").expect("query source"));
        assert_eq!(mastered_entry_ids(&conn).expect("ids").len(), 1);
    }
}
