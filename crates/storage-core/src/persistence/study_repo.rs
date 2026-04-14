//! Study repository for session persistence.

use rusqlite::Connection;

use crate::models::{SessionSummary, StudyResult, StudySession};
use crate::StorageError;

/// Save a completed study session with results.
pub fn save_completed_session(
    conn: &Connection,
    session: &StudySession,
    summary: &SessionSummary,
    results: &[StudyResult],
    next_action: &str,
) -> Result<(), StorageError> {
    // Insert session
    conn.execute(
        "INSERT INTO study_sessions (session_id, mode, total_words, wordbook_id, started_at, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(session_id) DO UPDATE SET
            completed_at = excluded.completed_at",
        rusqlite::params![
            session.session_id,
            serde_json::to_string(&session.mode).unwrap_or_default(),
            session.total_words,
            session.wordbook_id,
            session.started_at,
            summary.completed_at,
        ],
    )
    .map_err(|e| StorageError::Database(format!("Failed to save session: {e}")))?;

    // Insert results
    for result in results {
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
                                       normalized_response, correct_answer, outcome, response_time_ms, answered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                session.session_id,
                result.question_id,
                result.entry_source_id, // Using source_id as entry_id placeholder
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

    let _ = next_action;
    Ok(())
}

/// Get recent study sessions.
pub fn get_recent_sessions(conn: &Connection, limit: i64) -> Result<Vec<StudySession>, StorageError> {
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
            let mode = serde_json::from_str(&mode_str).unwrap_or_default();
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
