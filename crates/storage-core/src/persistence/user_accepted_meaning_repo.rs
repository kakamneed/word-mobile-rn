use rusqlite::{params, Connection, OptionalExtension};

use crate::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAcceptedMeaning {
    pub id: i64,
    pub entry_id: i64,
    pub entry_source_id: String,
    pub meaning_cn: String,
    pub source: String,
    pub question_id: String,
    pub question_type: String,
    pub submitted_answer: String,
    pub created_at: String,
}

pub fn list_for_entry(
    conn: &Connection,
    entry_id: i64,
) -> Result<Vec<UserAcceptedMeaning>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, entry_id, entry_source_id, meaning_cn, source,
                    question_id, question_type, submitted_answer, created_at
             FROM user_accepted_meanings
             WHERE entry_id = ?1 AND TRIM(meaning_cn) <> ''
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(|e| {
            StorageError::Database(format!("Failed to prepare accepted meaning query: {e}"))
        })?;
    let rows = stmt
        .query_map([entry_id], |row| {
            Ok(UserAcceptedMeaning {
                id: row.get(0)?,
                entry_id: row.get(1)?,
                entry_source_id: row.get(2)?,
                meaning_cn: row.get(3)?,
                source: row.get(4)?,
                question_id: row.get(5)?,
                question_type: row.get(6)?,
                submitted_answer: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| StorageError::Database(format!("Failed to query accepted meanings: {e}")))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| StorageError::Database(format!("Failed to decode accepted meaning row: {e}")))
}

pub fn save_user_dispute(
    conn: &Connection,
    entry_source_id: &str,
    question_id: &str,
    question_type: &str,
    submitted_answer: &str,
) -> Result<UserAcceptedMeaning, StorageError> {
    let meaning = submitted_answer.trim();
    if meaning.is_empty() {
        return Err(StorageError::Database(
            "Disputed accepted meaning cannot be empty".to_string(),
        ));
    }
    let entry_id = resolve_entry_id(conn, entry_source_id)?.ok_or_else(|| {
        StorageError::NotFound(format!("Entry not found for source id {entry_source_id}"))
    })?;
    conn.execute(
        "INSERT INTO user_accepted_meanings
            (entry_id, entry_source_id, meaning_cn, source, question_id, question_type,
             submitted_answer, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'user_dispute', ?4, ?5, ?6, datetime('now'), datetime('now'))
         ON CONFLICT(entry_id, meaning_cn) DO UPDATE SET
            source = excluded.source,
            question_id = excluded.question_id,
            question_type = excluded.question_type,
            submitted_answer = excluded.submitted_answer,
            updated_at = datetime('now')",
        params![
            entry_id,
            entry_source_id.trim(),
            meaning,
            question_id.trim(),
            question_type.trim(),
            submitted_answer.trim(),
        ],
    )
    .map_err(|e| StorageError::Database(format!("Failed to save accepted meaning: {e}")))?;

    conn.query_row(
        "SELECT id, entry_id, entry_source_id, meaning_cn, source,
                question_id, question_type, submitted_answer, created_at
         FROM user_accepted_meanings
         WHERE entry_id = ?1 AND meaning_cn = ?2
         LIMIT 1",
        params![entry_id, meaning],
        |row| {
            Ok(UserAcceptedMeaning {
                id: row.get(0)?,
                entry_id: row.get(1)?,
                entry_source_id: row.get(2)?,
                meaning_cn: row.get(3)?,
                source: row.get(4)?,
                question_id: row.get(5)?,
                question_type: row.get(6)?,
                submitted_answer: row.get(7)?,
                created_at: row.get(8)?,
            })
        },
    )
    .map_err(|e| StorageError::Database(format!("Failed to reload accepted meaning: {e}")))
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
        if exists.is_some() {
            return Ok(Some(entry_id));
        }
    }
    conn.query_row(
        "SELECT id FROM entries WHERE source_entry_key = ?1 ORDER BY id ASC LIMIT 1",
        [entry_source_id],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(|e| StorageError::Database(format!("Failed to resolve entry id: {e}")))
}
