//! Study session facade for learning loop.

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use word_storage_core::models::{
    CompleteSessionResponse, EntryExample, MeaningZh, SessionProgress, StartSessionEntryPayload,
    StartSessionRequest, StartSessionResponse, StudyQuestion, StudyResult, StudySession,
    SubmitAnswerRequest, SubmitAnswerResponse,
};
use word_storage_core::persistence;

use word_study_core::{AnswerEvaluator, QuestionBuilder, SessionSummaryService, WordForQuestion};

/// Errors that can occur during study operations.
#[derive(Debug, thiserror::Error)]
pub enum StudyError {
    #[error("No active session")]
    NoActiveSession,
    #[error("Session already active")]
    SessionAlreadyActive,
    #[error("Invalid mode: {0}")]
    InvalidMode(String),
    #[error("Not enough words")]
    NotEnoughWords,
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Active session state held in memory.
struct ActiveSession {
    session: StudySession,
    questions: Vec<StudyQuestion>,
    results: Vec<StudyResult>,
    current_index: usize,
    question_map: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveSessionSnapshot {
    #[serde(default = "default_question_engine_version")]
    question_engine_version: i64,
    session: StudySession,
    questions: Vec<StudyQuestion>,
    results: Vec<StudyResult>,
    current_index: usize,
}

// Global in-memory session storage
static ACTIVE_SESSIONS: Lazy<Mutex<HashMap<String, ActiveSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const QUESTION_ENGINE_VERSION: i64 = 4;

fn default_question_engine_version() -> i64 {
    0
}

/// Clear all active sessions from memory.
///
/// Primarily for test isolation: call before each test to prevent
/// global state leakage between parallel test runs.
pub fn clear_all_active_sessions() {
    let mut guard = ACTIVE_SESSIONS.lock().unwrap_or_else(|e| e.into_inner());
    guard.clear();
}

/// Lock ACTIVE_SESSIONS, recovering from mutex poison.
fn lock_sessions() -> std::sync::MutexGuard<'static, HashMap<String, ActiveSession>> {
    ACTIVE_SESSIONS.lock().unwrap_or_else(|e| e.into_inner())
}

/// Start a new study session.
///
/// # Example
///
/// ```rust,ignore
/// let response = start_study_session(&conn, StartSessionRequest {
///     mode: SessionMode::NewWord,
///     wordbook_id: Some(1),
///     entry_source_ids: vec!["word1".to_string(), "word2".to_string()],
/// })?;
/// ```
pub fn start_study_session(
    conn: &Connection,
    request: StartSessionRequest,
) -> Result<StartSessionResponse, StudyError> {
    let mode = request.mode.clone();
    let mode_key = mode_storage_key(&mode);
    let total_words = request.entry_source_ids.len() as u32;

    {
        let guard = lock_sessions();
        if let Some(active) = guard.get(&mode_key) {
            if active_session_matches_request_sources(active, &request) {
                return Ok(build_start_response(active));
            }
        }
    }

    if let Some(snapshot) = load_persisted_session(conn, &mode)? {
        let expected_questions =
            if request.entry_source_ids.is_empty() && request.entry_payloads.is_empty() {
                snapshot.questions.len()
            } else {
                expected_question_count(&mode, total_words as usize)
            };
        if snapshot.question_engine_version == QUESTION_ENGINE_VERSION
            && snapshot.questions.len() == expected_questions
            && snapshot.current_index < snapshot.questions.len()
            && snapshot_started_today(&snapshot)
            && !snapshot_contains_restore_placeholders(&snapshot)
            && snapshot_matches_request_sources(&snapshot, &request)
        {
            let active = active_session_from_snapshot(snapshot);
            let response = build_start_response(&active);
            let mut guard = lock_sessions();
            guard.insert(mode_key, active);
            return Ok(response);
        }
        clear_persisted_session(conn, &mode)?;
    }

    if total_words == 0 {
        return Err(StudyError::NotEnoughWords);
    }

    // Create words for question builder
    let words: Vec<WordForQuestion> = if !request.entry_payloads.is_empty() {
        payloads_to_words(&request.entry_payloads)
    } else {
        request
            .entry_source_ids
            .iter()
            .map(|id| WordForQuestion {
                source_id: id.clone(),
                word: id.clone(),
                part_of_speech: None,
                frequency: 0.0,
                phonetic_us: None,
                phonetic_uk: None,
                meanings: vec![],
                examples: vec![],
            })
            .collect()
    };

    let distractors: Vec<WordForQuestion> = if !request.distractor_payloads.is_empty() {
        payloads_to_words(&request.distractor_payloads)
    } else {
        Vec::new()
    };

    let now = chrono::Utc::now();
    let session_id = format!("sess_{}", now.format("%Y%m%d%H%M%S%f"));
    let started_at = now.to_rfc3339();

    // Generate questions
    let questions =
        QuestionBuilder::build_session_questions(&mode, &words, &distractors, &session_id);

    if questions.is_empty() {
        return Err(StudyError::NotEnoughWords);
    }

    let session = StudySession {
        session_id: session_id.clone(),
        mode: mode.clone(),
        total_words,
        wordbook_id: request.wordbook_id,
        started_at,
    };

    // Build question map
    let mut question_map = HashMap::new();
    for (i, q) in questions.iter().enumerate() {
        question_map.insert(q.question_id.clone(), i);
    }

    // Store active session
    let active = ActiveSession {
        session,
        questions: questions.clone(),
        results: Vec::new(),
        current_index: 0,
        question_map,
    };
    persist_active_session(conn, &active)?;

    let response = build_start_response(&active);
    {
        let mut guard = lock_sessions();
        guard.insert(mode_key, active);
    }

    Ok(response)
}

fn payloads_to_words(payloads: &[StartSessionEntryPayload]) -> Vec<WordForQuestion> {
    payloads
        .iter()
        .map(|payload| WordForQuestion {
            source_id: payload.source_id.clone(),
            word: payload.word.clone(),
            part_of_speech: payload.part_of_speech.clone(),
            frequency: payload.frequency,
            phonetic_us: payload.phonetic_us.clone(),
            phonetic_uk: payload.phonetic_uk.clone(),
            meanings: if payload.meaning_details.is_empty() {
                payload
                    .meanings
                    .iter()
                    .map(|meaning| MeaningZh {
                        pos: String::new(),
                        meaning_cn: meaning.clone(),
                        meaning_en: None,
                    })
                    .collect()
            } else {
                payload
                    .meaning_details
                    .iter()
                    .map(|meaning| MeaningZh {
                        pos: meaning.pos.clone(),
                        meaning_cn: meaning.meaning_cn.clone(),
                        meaning_en: meaning.meaning_en.clone(),
                    })
                    .collect()
            },
            examples: payload
                .example_sentence
                .iter()
                .map(|sentence| EntryExample {
                    sentence_en: sentence.clone(),
                    sentence_cn: payload.example_translation.clone().unwrap_or_default(),
                })
                .collect(),
        })
        .collect()
}

/// Get the currently active study session state without mutating it.
pub fn get_active_study_session() -> Result<StartSessionResponse, StudyError> {
    let guard = lock_sessions();
    let active = guard.values().next().ok_or(StudyError::NoActiveSession)?;
    Ok(build_start_response(active))
}

/// Submit an answer for the current question.
pub fn submit_study_answer(
    conn: &Connection,
    request: SubmitAnswerRequest,
) -> Result<SubmitAnswerResponse, StudyError> {
    let mut guard = lock_sessions();
    let active = guard
        .values_mut()
        .find(|session| session.question_map.contains_key(&request.question_id))
        .ok_or(StudyError::NoActiveSession)?;

    let current_q = &active.questions[active.current_index];
    let answered_at = chrono::Utc::now().to_rfc3339();

    // Create answer
    let answer = word_storage_core::models::StudyAnswer {
        question_id: request.question_id,
        response: request.response,
        response_time_ms: request.response_time_ms,
    };

    // Evaluate
    let result = AnswerEvaluator::evaluate(current_q, &answer, &answered_at);
    active.results.push(result.clone());

    // Advance
    active.current_index += 1;
    let is_complete = active.current_index >= active.questions.len();

    let (next_question, summary, next_action) = if is_complete {
        let completed_at = chrono::Utc::now().to_rfc3339();
        let summary =
            SessionSummaryService::build_summary(&active.session, &active.results, &completed_at);
        let next_action = SessionSummaryService::next_action(&summary, &active.session.mode);
        (None, Some(summary), Some(next_action))
    } else {
        (
            Some(active.questions[active.current_index].clone()),
            None,
            None,
        )
    };

    let response = SubmitAnswerResponse {
        result,
        is_complete,
        current_question: next_question,
        summary,
        next_action,
        progress: SessionProgress {
            current: if is_complete {
                active.questions.len() as u32
            } else {
                active.current_index as u32 + 1
            },
            total: active.questions.len() as u32,
        },
    };

    persist_active_session(conn, active)?;
    if let Err(error) =
        persistence::study_repo::save_session_progress(conn, &active.session, &active.results)
    {
        eprintln!("study progress persistence failed but active session was saved: {error}");
    }

    Ok(response)
}

/// Complete the active session and persist results.
pub fn complete_study_session(
    conn: &Connection,
    session_id: &str,
) -> Result<CompleteSessionResponse, StudyError> {
    let mut guard = lock_sessions();
    let mode_key = guard
        .iter()
        .find(|(_, active)| active.session.session_id == session_id)
        .map(|(mode, _)| mode.clone())
        .ok_or(StudyError::NoActiveSession)?;
    let active = guard.get(&mode_key).ok_or(StudyError::NoActiveSession)?;

    let completed_at = chrono::Utc::now().to_rfc3339();
    let summary =
        SessionSummaryService::build_summary(&active.session, &active.results, &completed_at);
    let next_action = SessionSummaryService::next_action(&summary, &active.session.mode);

    // Persist to database
    if let Err(error) = persistence::study_repo::save_completed_session(
        conn,
        &active.session,
        &summary,
        &active.results,
        &next_action,
    ) {
        eprintln!("study persistence failed but session will still complete: {error}");
    }

    clear_persisted_session(conn, &active.session.mode)?;
    guard.remove(&mode_key);

    Ok(CompleteSessionResponse {
        summary,
        next_action,
    })
}

/// Cancel the active session without persisting.
pub fn cancel_study_session(conn: &Connection, session_id: &str) -> Result<(), StudyError> {
    let mut guard = lock_sessions();
    let mode_key = guard
        .iter()
        .find(|(_, active)| active.session.session_id == session_id)
        .map(|(mode, _)| mode.clone())
        .ok_or(StudyError::NoActiveSession)?;
    let mode = guard
        .get(&mode_key)
        .map(|active| active.session.mode.clone())
        .ok_or(StudyError::NoActiveSession)?;
    guard.remove(&mode_key);
    clear_persisted_session(conn, &mode)?;
    Ok(())
}

fn build_start_response(active: &ActiveSession) -> StartSessionResponse {
    let index = active
        .current_index
        .min(active.questions.len().saturating_sub(1));
    let question = active.questions[index].clone();
    StartSessionResponse {
        session: active.session.clone(),
        current_question: question,
        progress: SessionProgress {
            current: index as u32 + 1,
            total: active.questions.len() as u32,
        },
    }
}

fn snapshot_started_today(snapshot: &ActiveSessionSnapshot) -> bool {
    chrono::DateTime::parse_from_rfc3339(&snapshot.session.started_at)
        .ok()
        .map(|started_at| {
            started_at.with_timezone(&chrono::Local).date_naive()
                == chrono::Local::now().date_naive()
        })
        .unwrap_or(false)
}

fn snapshot_contains_restore_placeholders(snapshot: &ActiveSessionSnapshot) -> bool {
    snapshot.questions.iter().any(|question| {
        question
            .entry_source_id
            .starts_with("active_session_restore_")
            || question.word.starts_with("active_session_restore_")
            || question.prompt.starts_with("active_session_restore_")
    })
}

fn snapshot_matches_request_sources(
    snapshot: &ActiveSessionSnapshot,
    request: &StartSessionRequest,
) -> bool {
    if request.entry_source_ids.is_empty() {
        return true;
    }
    let requested = request
        .entry_source_ids
        .iter()
        .collect::<std::collections::HashSet<_>>();
    snapshot
        .questions
        .iter()
        .all(|question| requested.contains(&question.entry_source_id))
}

fn active_session_matches_request_sources(
    active: &ActiveSession,
    request: &StartSessionRequest,
) -> bool {
    if request.entry_source_ids.is_empty() {
        return true;
    }
    let requested = request
        .entry_source_ids
        .iter()
        .collect::<std::collections::HashSet<_>>();
    active
        .questions
        .iter()
        .all(|question| requested.contains(&question.entry_source_id))
}

fn mode_storage_key(mode: &word_storage_core::models::SessionMode) -> String {
    serde_json::to_string(mode).unwrap_or_else(|_| "unknown".to_string())
}

fn active_session_from_snapshot(snapshot: ActiveSessionSnapshot) -> ActiveSession {
    let mut question_map = HashMap::new();
    for (i, question) in snapshot.questions.iter().enumerate() {
        question_map.insert(question.question_id.clone(), i);
    }
    ActiveSession {
        session: snapshot.session,
        questions: snapshot.questions,
        results: snapshot.results,
        current_index: snapshot.current_index,
        question_map,
    }
}

fn load_persisted_session(
    conn: &Connection,
    mode: &word_storage_core::models::SessionMode,
) -> Result<Option<ActiveSessionSnapshot>, StudyError> {
    let raw = persistence::study_repo::load_active_session_snapshot(conn, mode)
        .map_err(|e| StudyError::Storage(e.to_string()))?;
    raw.map(|value| {
        serde_json::from_str(&value)
            .map_err(|e| StudyError::Storage(format!("Invalid active session snapshot: {e}")))
    })
    .transpose()
}

fn persist_active_session(conn: &Connection, active: &ActiveSession) -> Result<(), StudyError> {
    let snapshot = ActiveSessionSnapshot {
        question_engine_version: QUESTION_ENGINE_VERSION,
        session: active.session.clone(),
        questions: active.questions.clone(),
        results: active.results.clone(),
        current_index: active.current_index,
    };
    let payload = serde_json::to_string(&snapshot)
        .map_err(|e| StudyError::Storage(format!("Failed to serialize active session: {e}")))?;
    persistence::study_repo::save_active_session_snapshot(conn, &active.session.mode, &payload)
        .map_err(|e| StudyError::Storage(e.to_string()))
}

fn clear_persisted_session(
    conn: &Connection,
    mode: &word_storage_core::models::SessionMode,
) -> Result<(), StudyError> {
    persistence::study_repo::delete_active_session_snapshot(conn, mode)
        .map_err(|e| StudyError::Storage(e.to_string()))
}

fn expected_question_count(
    mode: &word_storage_core::models::SessionMode,
    total_words: usize,
) -> usize {
    match mode {
        word_storage_core::models::SessionMode::NewWord
        | word_storage_core::models::SessionMode::Review => total_words * 4,
        _ => total_words,
    }
}

#[cfg(test)]
mod tests {
    use super::{clear_all_active_sessions, start_study_session};
    use word_storage_core::models::{
        ChoiceOption, QuestionType, SessionMode, StartSessionEntryPayload,
        StartSessionMeaningPayload, StartSessionRequest, StudyQuestion,
    };

    #[test]
    fn start_session_discards_restore_placeholder_snapshot_and_rebuilds_from_payloads() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();
        let started_at = chrono::Local::now().to_rfc3339();

        let snapshot = serde_json::json!({
            "questionEngineVersion": 4,
            "session": {
                "sessionId": "sess_restore_old",
                "mode": "rootAffix",
                "totalWords": 1,
                "wordbookId": null,
                "startedAt": started_at
            },
            "questions": [serde_json::to_value(StudyQuestion {
                question_id: "sess_restore_old_0".to_string(),
                question_type: QuestionType::RootToGlossInput,
                entry_source_id: "active_session_restore_0".to_string(),
                word: "active_session_restore_0".to_string(),
                part_of_speech: None,
                phonetic_us: None,
                phonetic_uk: None,
                prompt: "active_session_restore_0".to_string(),
                accepted_meanings: vec!["placeholder".to_string()],
                example_sentence: None,
                example_translation: None,
                choices: Some(vec![ChoiceOption {
                    text: "placeholder".to_string(),
                    label: "A".to_string(),
                }]),
                correct_choice_label: Some("A".to_string()),
                question_index: 0,
                total_questions: 1,
            }).expect("serialize question")],
            "results": [],
            "currentIndex": 0
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_rootAffix", snapshot.to_string()),
        )
        .expect("insert placeholder snapshot");

        let response = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::RootAffix,
                wordbook_id: None,
                entry_source_ids: vec!["root_affix_re".to_string()],
                entry_payloads: vec![StartSessionEntryPayload {
                    source_id: "root_affix_re".to_string(),
                    word: "re-".to_string(),
                    part_of_speech: None,
                    frequency: 0.0,
                    phonetic_us: None,
                    phonetic_uk: None,
                    meaning_details: vec![StartSessionMeaningPayload {
                        pos: "root".to_string(),
                        meaning_cn: "again".to_string(),
                        meaning_en: None,
                    }],
                    meanings: vec!["again".to_string()],
                    example_sentence: None,
                    example_translation: None,
                }],
                distractor_payloads: Vec::new(),
            },
        )
        .expect("start session should rebuild");

        assert_eq!(response.current_question.entry_source_id, "root_affix_re");
        assert_eq!(response.current_question.word, "re-");
        assert_ne!(response.session.session_id, "sess_restore_old");
    }

    #[test]
    fn start_session_discards_same_day_snapshot_when_sources_changed() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();
        let started_at = chrono::Local::now().to_rfc3339();

        let snapshot = serde_json::json!({
            "questionEngineVersion": 4,
            "session": {
                "sessionId": "sess_root_old_order",
                "mode": "rootAffix",
                "totalWords": 1,
                "wordbookId": null,
                "startedAt": started_at
            },
            "questions": [serde_json::to_value(StudyQuestion {
                question_id: "sess_root_old_order_0".to_string(),
                question_type: QuestionType::RootToGlossInput,
                entry_source_id: "root_affix_shared_ab".to_string(),
                word: "ab-".to_string(),
                part_of_speech: None,
                phonetic_us: None,
                phonetic_uk: None,
                prompt: "ab-".to_string(),
                accepted_meanings: vec!["old".to_string()],
                example_sentence: None,
                example_translation: None,
                choices: None,
                correct_choice_label: None,
                question_index: 0,
                total_questions: 1,
            }).expect("serialize question")],
            "results": [],
            "currentIndex": 0
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_rootAffix", snapshot.to_string()),
        )
        .expect("insert stale same-day snapshot");

        let response = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::RootAffix,
                wordbook_id: None,
                entry_source_ids: vec!["root_affix_shared_trans".to_string()],
                entry_payloads: vec![StartSessionEntryPayload {
                    source_id: "root_affix_shared_trans".to_string(),
                    word: "trans-".to_string(),
                    part_of_speech: None,
                    frequency: 0.0,
                    phonetic_us: None,
                    phonetic_uk: None,
                    meaning_details: vec![StartSessionMeaningPayload {
                        pos: "root".to_string(),
                        meaning_cn: "across".to_string(),
                        meaning_en: None,
                    }],
                    meanings: vec!["across".to_string()],
                    example_sentence: None,
                    example_translation: None,
                }],
                distractor_payloads: Vec::new(),
            },
        )
        .expect("start session should rebuild from changed sources");

        assert_eq!(
            response.current_question.entry_source_id,
            "root_affix_shared_trans"
        );
        assert_eq!(response.current_question.word, "trans-");
        assert_ne!(response.session.session_id, "sess_root_old_order");
    }

    #[test]
    fn start_session_rebuilds_in_memory_session_when_sources_changed() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let first = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::NewWord,
                wordbook_id: Some(1),
                entry_source_ids: vec!["book_one_word".to_string()],
                entry_payloads: vec![StartSessionEntryPayload {
                    source_id: "book_one_word".to_string(),
                    word: "alpha".to_string(),
                    part_of_speech: None,
                    frequency: 1.0,
                    phonetic_us: None,
                    phonetic_uk: None,
                    meaning_details: vec![StartSessionMeaningPayload {
                        pos: "n.".to_string(),
                        meaning_cn: "alpha meaning".to_string(),
                        meaning_en: None,
                    }],
                    meanings: vec!["alpha meaning".to_string()],
                    example_sentence: None,
                    example_translation: None,
                }],
                distractor_payloads: Vec::new(),
            },
        )
        .expect("start first session");

        let second = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::NewWord,
                wordbook_id: Some(2),
                entry_source_ids: vec!["book_two_word".to_string()],
                entry_payloads: vec![StartSessionEntryPayload {
                    source_id: "book_two_word".to_string(),
                    word: "beta".to_string(),
                    part_of_speech: None,
                    frequency: 1.0,
                    phonetic_us: None,
                    phonetic_uk: None,
                    meaning_details: vec![StartSessionMeaningPayload {
                        pos: "n.".to_string(),
                        meaning_cn: "beta meaning".to_string(),
                        meaning_en: None,
                    }],
                    meanings: vec!["beta meaning".to_string()],
                    example_sentence: None,
                    example_translation: None,
                }],
                distractor_payloads: Vec::new(),
            },
        )
        .expect("start second session from changed sources");

        assert_eq!(first.current_question.entry_source_id, "book_one_word");
        assert_eq!(second.current_question.entry_source_id, "book_two_word");
        assert_eq!(second.session.wordbook_id, Some(2));
        assert_ne!(first.session.session_id, second.session.session_id);
    }

    #[test]
    fn start_session_restores_same_day_snapshot_for_empty_resume_request() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();
        let started_at = chrono::Local::now().to_rfc3339();
        let questions = (0..32)
            .map(|index| {
                serde_json::to_value(StudyQuestion {
                    question_id: format!("sess_resume_{index}"),
                    question_type: QuestionType::EnToCnChoice,
                    entry_source_id: format!("entry_{}", index / 4),
                    word: format!("word_{}", index / 4),
                    part_of_speech: None,
                    phonetic_us: None,
                    phonetic_uk: None,
                    prompt: format!("word_{}", index / 4),
                    accepted_meanings: vec![format!("meaning_{}", index / 4)],
                    example_sentence: None,
                    example_translation: None,
                    choices: Some(vec![ChoiceOption {
                        text: format!("meaning_{}", index / 4),
                        label: "A".to_string(),
                    }]),
                    correct_choice_label: Some("A".to_string()),
                    question_index: index as u32,
                    total_questions: 32,
                })
                .expect("serialize question")
            })
            .collect::<Vec<_>>();

        let snapshot = serde_json::json!({
            "questionEngineVersion": 4,
            "session": {
                "sessionId": "sess_resume",
                "mode": "newWord",
                "totalWords": 8,
                "wordbookId": null,
                "startedAt": started_at
            },
            "questions": questions,
            "results": [],
            "currentIndex": 8
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_newWord", snapshot.to_string()),
        )
        .expect("insert active snapshot");

        let response = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::NewWord,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
        )
        .expect("empty start request should restore saved progress");

        assert_eq!(response.session.session_id, "sess_resume");
        assert_eq!(response.progress.current, 9);
        assert_eq!(response.progress.total, 32);
        assert_eq!(response.current_question.question_id, "sess_resume_8");
    }
}
