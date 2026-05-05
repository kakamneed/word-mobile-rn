use rusqlite::{params, Connection, OptionalExtension};

use crate::StorageError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordHint {
    pub entry_id: i64,
    pub hint_text: String,
    pub source: String,
    pub updated_at: String,
}

pub fn get_hint(conn: &Connection, entry_id: i64) -> Result<Option<WordHint>, StorageError> {
    conn.query_row(
        "SELECT entry_id, hint_text, source, updated_at
         FROM word_hints
         WHERE entry_id = ?1 AND TRIM(hint_text) <> ''",
        [entry_id],
        |row| {
            Ok(WordHint {
                entry_id: row.get(0)?,
                hint_text: row.get(1)?,
                source: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| StorageError::Database(format!("Failed to load word hint: {e}")))
}

pub fn save_hint(
    conn: &Connection,
    entry_id: i64,
    hint_text: &str,
    source: &str,
) -> Result<Option<WordHint>, StorageError> {
    let normalized = hint_text.trim();
    if normalized.is_empty() {
        clear_hint(conn, entry_id)?;
        return Ok(None);
    }

    conn.execute(
        "INSERT INTO word_hints (entry_id, hint_text, source, created_at, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'), datetime('now'))
         ON CONFLICT(entry_id) DO UPDATE SET
            hint_text = excluded.hint_text,
            source = excluded.source,
            updated_at = datetime('now')",
        params![entry_id, normalized, normalize_source(source)],
    )
    .map_err(|e| StorageError::Database(format!("Failed to save word hint: {e}")))?;

    get_hint(conn, entry_id)
}

pub fn clear_hint(conn: &Connection, entry_id: i64) -> Result<(), StorageError> {
    conn.execute("DELETE FROM word_hints WHERE entry_id = ?1", [entry_id])
        .map_err(|e| StorageError::Database(format!("Failed to clear word hint: {e}")))?;
    Ok(())
}

pub fn counted_error_count(conn: &Connection, entry_id: i64) -> Result<i64, StorageError> {
    conn.query_row(
        "SELECT COUNT(*)
         FROM study_results
         WHERE entry_id = ?1
           AND outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"')",
        [entry_id],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|e| StorageError::Database(format!("Failed to count hint-trigger errors: {e}")))
}

fn normalize_source(source: &str) -> &str {
    match source.trim() {
        "aiSuggestion" => "aiSuggestion",
        _ => "user",
    }
}

#[cfg(test)]
mod tests {
    use super::{clear_hint, counted_error_count, get_hint, save_hint};

    fn seed_entry(conn: &rusqlite::Connection) -> i64 {
        word_storage_core_test_support::seed_entry(conn)
    }

    mod word_storage_core_test_support {
        pub fn seed_entry(conn: &rusqlite::Connection) -> i64 {
            crate::persistence::schema::apply_schema(conn).expect("apply schema");
            conn.execute(
                "INSERT INTO source_versions (source_commit, status) VALUES ('hint-test-v1', 'ready')",
                [],
            )
            .expect("insert source version");
            let source_version_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO entries (source_version_id, source_entry_key, word, lemma)
                 VALUES (?1, 'hint_alpha', 'alpha', 'alpha')",
                [source_version_id],
            )
            .expect("insert entry");
            conn.last_insert_rowid()
        }
    }

    #[test]
    fn saves_updates_and_clears_hint() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        let entry_id = seed_entry(&conn);

        let saved = save_hint(&conn, entry_id, "  a first hint  ", "user")
            .expect("save hint")
            .expect("hint exists");
        assert_eq!(saved.hint_text, "a first hint");
        assert_eq!(saved.source, "user");

        let updated = save_hint(&conn, entry_id, "AI style hint", "aiSuggestion")
            .expect("update hint")
            .expect("hint exists");
        assert_eq!(updated.hint_text, "AI style hint");
        assert_eq!(updated.source, "aiSuggestion");

        clear_hint(&conn, entry_id).expect("clear hint");
        assert!(get_hint(&conn, entry_id).expect("load hint").is_none());
    }

    #[test]
    fn counted_error_count_includes_incorrect_and_skipped_only() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        let entry_id = seed_entry(&conn);
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at)
             VALUES ('s1', 'newWord', 1, '2026-05-04T00:00:00Z')",
            [],
        )
        .expect("insert session");
        for (index, outcome) in ["incorrect", "skipped", "correct", "fuzzyCorrect"]
            .iter()
            .enumerate()
        {
            conn.execute(
                "INSERT INTO study_results
                 (session_id, question_id, entry_id, question_type, user_response,
                  correct_answer, outcome, response_time_ms, answered_at)
                 VALUES ('s1', ?1, ?2, 'enToCnChoice', '', '', ?3, 1, '2026-05-04T00:00:00Z')",
                rusqlite::params![format!("q{index}"), entry_id, outcome],
            )
            .expect("insert result");
        }

        assert_eq!(counted_error_count(&conn, entry_id).expect("count"), 2);
    }
}
