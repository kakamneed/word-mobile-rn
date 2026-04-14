//! Entry repository for vocabulary entries.

use rusqlite::Connection;

use crate::models::StandardizedEntry;
use crate::StorageError;

/// Insert an entry.
pub fn insert_entry(
    conn: &Connection,
    source_version_id: i64,
    source_entry_key: &str,
    word: &str,
    lemma: &str,
    phonetic_us: Option<&str>,
    phonetic_uk: Option<&str>,
    part_of_speech: Option<&str>,
    frequency: f64,
    difficulty: Option<&str>,
) -> Result<i64, StorageError> {
    conn.execute(
        "INSERT INTO entries (source_version_id, source_entry_key, word, lemma, phonetic_us, phonetic_uk, part_of_speech, frequency, difficulty)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(source_version_id, source_entry_key) DO UPDATE SET
            word = excluded.word,
            lemma = excluded.lemma,
            phonetic_us = excluded.phonetic_us,
            phonetic_uk = excluded.phonetic_uk,
            part_of_speech = excluded.part_of_speech,
            frequency = excluded.frequency,
            difficulty = excluded.difficulty",
        rusqlite::params![
            source_version_id,
            source_entry_key,
            word,
            lemma,
            phonetic_us,
            phonetic_uk,
            part_of_speech,
            frequency,
            difficulty,
        ],
    )
    .map_err(|e| StorageError::Database(format!("Failed to upsert entry: {e}")))?;

    Ok(conn.last_insert_rowid())
}

/// Get entries by source IDs.
pub fn get_entries_by_source_ids(
    conn: &Connection,
    source_ids: &[String],
) -> Result<Vec<StandardizedEntry>, StorageError> {
    if source_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders: Vec<String> = source_ids.iter().enumerate().map(|(i, _)| format!("?{}", i + 1)).collect();
    let sql = format!(
        "SELECT source_entry_key, word, lemma, phonetic_us, phonetic_uk, part_of_speech, frequency, difficulty
         FROM entries
         WHERE source_entry_key IN ({})",
        placeholders.join(",")
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| StorageError::Database(format!("Failed to prepare: {e}")))?;

    let param_refs: Vec<&dyn rusqlite::ToSql> = source_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let rows = stmt
        .query_map(&param_refs[..], |row| {
            Ok(StandardizedEntry {
                source_id: row.get(0)?,
                word: row.get(1)?,
                lemma: row.get(2)?,
                phonetic_us: row.get(3)?,
                phonetic_uk: row.get(4)?,
                part_of_speech: row.get(5)?,
                frequency: row.get(6)?,
                difficulty: row.get(7)?,
                meanings_zh: vec![],
                examples: vec![],
                tags: vec![],
            })
        })
        .map_err(|e| StorageError::Database(format!("Failed to query: {e}")))?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row.map_err(|e| StorageError::Database(format!("Failed to read row: {e}")))?);
    }

    Ok(entries)
}
