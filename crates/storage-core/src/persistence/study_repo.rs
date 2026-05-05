//! Study repository for session persistence.

use rusqlite::{Connection, OptionalExtension};

use crate::models::{SessionMode, SessionSummary, StudyResult, StudySession};
use crate::StorageError;

/// Save a completed study session with results.
pub fn save_completed_session(
    conn: &Connection,
    session: &StudySession,
    summary: &SessionSummary,
    results: &[StudyResult],
    next_action: &str,
) -> Result<(), StorageError> {
    save_session_results(conn, session, Some(&summary.completed_at), results)?;
    let _ = next_action;
    Ok(())
}

/// Save the current answered results for an unfinished active session.
///
/// This keeps wrong-word and AI-context surfaces in sync even when the user
/// leaves a session before the final summary screen.
pub fn save_session_progress(
    conn: &Connection,
    session: &StudySession,
    results: &[StudyResult],
) -> Result<(), StorageError> {
    save_session_results(conn, session, None, results)
}

fn save_session_results(
    conn: &Connection,
    session: &StudySession,
    completed_at: Option<&str>,
    results: &[StudyResult],
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO study_sessions (session_id, mode, total_words, wordbook_id, started_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(session_id) DO UPDATE SET
            total_words = excluded.total_words,
            wordbook_id = excluded.wordbook_id,
            completed_at = excluded.completed_at",
        rusqlite::params![
            session.session_id,
            serde_json::to_string(&session.mode).unwrap_or_default(),
            session.total_words,
            session.wordbook_id,
            session.started_at,
            completed_at,
        ],
    )
    .map_err(|e| StorageError::Database(format!("Failed to save session: {e}")))?;

    conn.execute(
        "DELETE FROM study_results WHERE session_id = ?1",
        rusqlite::params![session.session_id],
    )
    .map_err(|e| StorageError::Database(format!("Failed to replace session results: {e}")))?;

    // Insert results
    for result in results {
        let Some(entry_id) = resolve_result_entry_id(conn, &result.entry_source_id)? else {
            continue;
        };
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
                                       normalized_response, correct_answer, outcome, response_time_ms, answered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                session.session_id,
                result.question_id,
                entry_id,
                serde_json::to_string(&result.question_type).unwrap_or_default(),
                result.user_response,
                result.normalized_response,
                result.correct_answer,
                serde_json::to_string(&result.outcome).unwrap_or_default(),
                result.response_time_ms,
                result.answered_at,
            ],
        )
        .map_err(|e| StorageError::Database(format!("Failed to save result: {e}")))?;
    }

    Ok(())
}

fn resolve_result_entry_id(
    conn: &Connection,
    source_entry_id: &str,
) -> Result<Option<i64>, StorageError> {
    if let Ok(entry_id) = source_entry_id.parse::<i64>() {
        let exists = conn
            .query_row(
                "SELECT 1 FROM entries WHERE id = ?1 LIMIT 1",
                [entry_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| {
                StorageError::Database(format!("Failed to verify result entry id: {e}"))
            })?;
        return Ok(exists.map(|_| entry_id));
    }
    let id = conn
        .query_row(
            "SELECT id FROM entries WHERE source_entry_key = ?1 ORDER BY id ASC LIMIT 1",
            [source_entry_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| StorageError::Database(format!("Failed to resolve result entry id: {e}")))?;
    if id.is_some() {
        return Ok(id);
    }
    if source_entry_id.starts_with("root_affix_") {
        return ensure_virtual_root_affix_entry_id(conn, source_entry_id);
    }
    Ok(None)
}

fn ensure_virtual_root_affix_entry_id(
    conn: &Connection,
    source_entry_id: &str,
) -> Result<Option<i64>, StorageError> {
    conn.execute(
        "INSERT OR IGNORE INTO source_versions (source_name, source_commit, status, notes)
         VALUES ('word-mobile/root-affix', 'root-affix-virtual-v1', 'ready', 'Internal root/affix study cards')",
        [],
    )
    .map_err(|e| {
        StorageError::Database(format!("Failed to ensure root/affix source version: {e}"))
    })?;

    let source_version_id = conn
        .query_row(
            "SELECT id FROM source_versions WHERE source_commit = 'root-affix-virtual-v1'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| {
            StorageError::Database(format!("Failed to load root/affix source version: {e}"))
        })?;

    let display_word = source_entry_id
        .strip_prefix("root_affix_shared_")
        .or_else(|| source_entry_id.strip_prefix("root_affix_medical_"))
        .unwrap_or(source_entry_id)
        .replace('_', "-");

    conn.execute(
        "INSERT OR IGNORE INTO entries (source_version_id, source_entry_key, word, lemma, part_of_speech)
         VALUES (?1, ?2, ?3, ?3, 'root')",
        rusqlite::params![source_version_id, source_entry_id, display_word],
    )
    .map_err(|e| StorageError::Database(format!("Failed to save root/affix entry: {e}")))?;

    conn.query_row(
        "SELECT id FROM entries WHERE source_version_id = ?1 AND source_entry_key = ?2",
        rusqlite::params![source_version_id, source_entry_id],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(|e| StorageError::Database(format!("Failed to resolve root/affix entry: {e}")))
}

/// Get recent study sessions.
pub fn get_recent_sessions(
    conn: &Connection,
    limit: i64,
) -> Result<Vec<StudySession>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT session_id, mode, total_words, wordbook_id, started_at
             FROM study_sessions
             ORDER BY started_at DESC
             LIMIT ?1",
        )
        .map_err(|e| StorageError::Database(format!("Failed to prepare: {e}")))?;

    let rows = stmt
        .query_map([limit], |row| {
            let mode_str: String = row.get(1)?;
            let mode = serde_json::from_str(&mode_str).unwrap_or(SessionMode::NewWord);
            Ok(StudySession {
                session_id: row.get(0)?,
                mode,
                total_words: row.get(2)?,
                wordbook_id: row.get(3)?,
                started_at: row.get(4)?,
            })
        })
        .map_err(|e| StorageError::Database(format!("Failed to query: {e}")))?;

    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(row.map_err(|e| StorageError::Database(format!("Failed to read row: {e}")))?);
    }

    Ok(sessions)
}

pub fn save_active_session_snapshot(
    conn: &Connection,
    mode: &SessionMode,
    snapshot_json: &str,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO app_settings (key, value_json, updated_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        rusqlite::params![active_session_key(mode), snapshot_json],
    )
    .map_err(|e| StorageError::Database(format!("Failed to save active session snapshot: {e}")))?;
    Ok(())
}

pub fn load_active_session_snapshot(
    conn: &Connection,
    mode: &SessionMode,
) -> Result<Option<String>, StorageError> {
    conn.query_row(
        "SELECT value_json FROM app_settings WHERE key = ?1",
        rusqlite::params![active_session_key(mode)],
        |row| row.get(0),
    )
    .optional()
    .map_err(|e| StorageError::Database(format!("Failed to load active session snapshot: {e}")))
}

pub fn delete_active_session_snapshot(
    conn: &Connection,
    mode: &SessionMode,
) -> Result<(), StorageError> {
    conn.execute(
        "DELETE FROM app_settings WHERE key = ?1",
        rusqlite::params![active_session_key(mode)],
    )
    .map_err(|e| {
        StorageError::Database(format!("Failed to delete active session snapshot: {e}"))
    })?;
    Ok(())
}

fn active_session_key(mode: &SessionMode) -> String {
    let suffix = match mode {
        SessionMode::NewWord => "newWord",
        SessionMode::Review => "review",
        SessionMode::MixedTest => "mixedTest",
        SessionMode::WrongWordReinforcement => "wrongWordReinforcement",
        SessionMode::RootAffix => "rootAffix",
    };
    format!("active_study_session_{suffix}")
}
