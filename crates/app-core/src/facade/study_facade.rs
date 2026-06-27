//! Study session facade for learning loop.

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use word_storage_core::models::{
    AcceptDisputedMeaningRequest, AcceptDisputedMeaningResponse, AnswerOutcome,
    AnsweredStudyQuestion, CompleteSessionResponse, EntryExample, MarkStudyEntryMasteredRequest,
    MarkStudyEntryMasteredResponse, MeaningZh, QuestionTypeWeight, SessionProgress,
    StartSessionEntryPayload, StartSessionRequest, StartSessionResponse, StudyQuestion,
    StudyResult, StudySession, SubmitAnswerRequest, SubmitAnswerResponse,
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
    question_type_weights: Vec<QuestionTypeWeight>,
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
    #[serde(default)]
    question_type_weights: Vec<QuestionTypeWeight>,
}

// Global in-memory session storage
static ACTIVE_SESSIONS: Lazy<Mutex<HashMap<String, ActiveSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const QUESTION_ENGINE_VERSION: i64 = 9;

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
    let question_type_weights =
        normalized_question_type_weights_for_session(&mode, &request.question_type_weights);
    let mut cleared_stale_persisted_snapshot = false;

    if !request.entry_source_ids.is_empty() || !request.entry_payloads.is_empty() {
        let guard = lock_sessions();
        if let Some(active) = guard.get(&mode_key) {
            if active.current_index < active.questions.len()
                && active_session_matches_request(active, &request, &question_type_weights)
            {
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
        let engine_stale = snapshot.question_engine_version != QUESTION_ENGINE_VERSION;
        let shape_stale = snapshot.questions.len() != expected_questions
            || snapshot.current_index >= snapshot.questions.len()
            || snapshot_contains_restore_placeholders(&snapshot);
        if !engine_stale
            && !shape_stale
            && snapshot_matches_request(&snapshot, &request, &question_type_weights)
        {
            let active = active_session_from_snapshot(snapshot);
            let response = build_start_response(&active);
            let mut guard = lock_sessions();
            guard.insert(mode_key, active);
            return Ok(response);
        }
        if engine_stale || shape_stale {
            clear_persisted_session(conn, &mode)?;
            cleared_stale_persisted_snapshot = true;
        }
        // Only clear the persisted snapshot if it is stale (not from today).
        // This preserves progress for same-day resume even when question-type
        // weights change across restarts.
        else if (!request.entry_source_ids.is_empty() || !request.entry_payloads.is_empty())
            && snapshot_question_weights_stale(&snapshot, &question_type_weights)
        {
            clear_persisted_session(conn, &mode)?;
        }
    }

    if request.entry_source_ids.is_empty() && request.entry_payloads.is_empty() {
        if cleared_stale_persisted_snapshot {
            return Err(StudyError::NotEnoughWords);
        }
        let guard = lock_sessions();
        if let Some(active) = guard.get(&mode_key) {
            if active.current_index < active.questions.len() {
                return Ok(build_start_response(active));
            }
        }
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
                cn_choice_distractors: Vec::new(),
                en_choice_distractors: Vec::new(),
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
    let questions = QuestionBuilder::build_session_questions(
        &mode,
        &words,
        &distractors,
        &session_id,
        &request.question_type_weights,
    );

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
        question_type_weights,
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
            cn_choice_distractors: payload.cn_choice_distractors.clone(),
            en_choice_distractors: payload.en_choice_distractors.clone(),
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
pub fn accept_disputed_meaning(
    conn: &Connection,
    request: AcceptDisputedMeaningRequest,
) -> Result<AcceptDisputedMeaningResponse, StudyError> {
    let mut guard = lock_sessions();
    let active = guard
        .values_mut()
        .find(|session| session.question_map.contains_key(&request.question_id))
        .ok_or(StudyError::NoActiveSession)?;
    let question_index = active
        .question_map
        .get(&request.question_id)
        .copied()
        .ok_or(StudyError::NoActiveSession)?;
    let question = active.questions[question_index].clone();
    let accepted_meaning = request.submitted_answer.trim().to_string();
    if accepted_meaning.is_empty() {
        return Err(StudyError::InvalidMode(
            "accepted meaning cannot be empty".to_string(),
        ));
    }
    let answered_at = chrono::Utc::now().to_rfc3339();
    let result = StudyResult {
        question_id: request.question_id,
        entry_source_id: question.entry_source_id.clone(),
        question_type: question.question_type.clone(),
        user_response: accepted_meaning.clone(),
        normalized_response: Some(accepted_meaning.clone()),
        correct_answer: accepted_meaning.clone(),
        outcome: AnswerOutcome::FuzzyCorrect,
        response_time_ms: 0,
        answered_at,
    };

    if let Some(existing) = active
        .results
        .iter_mut()
        .find(|item| item.question_id == result.question_id)
    {
        *existing = result.clone();
    } else {
        active.results.push(result.clone());
    }
    persist_active_session(conn, active)?;
    if let Err(error) =
        persistence::study_repo::save_session_progress(conn, &active.session, &active.results)
    {
        eprintln!("study progress persistence failed after disputed meaning accept: {error}");
    }

    Ok(AcceptDisputedMeaningResponse {
        result,
        progress: SessionProgress {
            current: (active.current_index as u32 + 1).min(active.questions.len() as u32),
            total: active.questions.len() as u32,
        },
        answered_questions: answered_questions(active),
        entry_source_id: question.entry_source_id,
        word: question.word,
        accepted_meaning,
    })
}
pub fn submit_study_answer(
    conn: &Connection,
    request: SubmitAnswerRequest,
) -> Result<SubmitAnswerResponse, StudyError> {
    let mut guard = lock_sessions();
    let active = guard
        .values_mut()
        .find(|session| session.question_map.contains_key(&request.question_id))
        .ok_or(StudyError::NoActiveSession)?;

    let requested_question_id = request.question_id.clone();
    let requested_index = active
        .question_map
        .get(&requested_question_id)
        .copied()
        .ok_or(StudyError::NoActiveSession)?;
    if requested_index != active.current_index {
        return Err(StudyError::NoActiveSession);
    }

    let current_q = &active.questions[requested_index];
    let answered_at = chrono::Utc::now().to_rfc3339();

    // Create answer
    let answer = word_storage_core::models::StudyAnswer {
        question_id: requested_question_id,
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
        answered_questions: answered_questions(active),
    };

    if is_complete {
        if let Some(summary) = response.summary.as_ref() {
            if let Some(next_action) = response.next_action.as_ref() {
                if let Err(error) = persistence::study_repo::save_completed_session(
                    conn,
                    &active.session,
                    summary,
                    &active.results,
                    next_action,
                ) {
                    eprintln!("study persistence failed but submitted session will still complete: {error}");
                }
            }
        }
        let mode_key = mode_storage_key(&active.session.mode);
        clear_persisted_session(conn, &active.session.mode)?;
        guard.remove(&mode_key);
    } else {
        persist_active_session(conn, active)?;
        if let Err(error) =
            persistence::study_repo::save_session_progress(conn, &active.session, &active.results)
        {
            eprintln!("study progress persistence failed but active session was saved: {error}");
        }
    }

    Ok(response)
}

pub fn mark_study_entry_mastered(
    conn: &Connection,
    request: MarkStudyEntryMasteredRequest,
) -> Result<MarkStudyEntryMasteredResponse, StudyError> {
    let entry_id = persistence::mastered_entry_repo::mark_mastered_by_source_id(
        conn,
        &request.entry_source_id,
        &request.reason,
    )
    .map_err(|e| StudyError::Storage(e.to_string()))?;

    let mut guard = lock_sessions();
    let mode_key = guard
        .iter()
        .find(|(_, active)| {
            active.questions.iter().any(|question| {
                question.entry_source_id == request.entry_source_id
                    && !active
                        .results
                        .iter()
                        .any(|result| result.question_id == question.question_id)
            })
        })
        .map(|(mode, _)| mode.clone())
        .ok_or(StudyError::NoActiveSession)?;
    let active = guard
        .get_mut(&mode_key)
        .ok_or(StudyError::NoActiveSession)?;
    let answered_ids = active
        .results
        .iter()
        .map(|result| result.question_id.clone())
        .collect::<std::collections::HashSet<_>>();
    let before = active.questions.len();
    active.questions.retain(|question| {
        question.entry_source_id != request.entry_source_id
            || answered_ids.contains(&question.question_id)
    });
    let pruned_question_count = before.saturating_sub(active.questions.len()) as u32;
    rebuild_question_map(active);
    active.current_index = active
        .questions
        .iter()
        .position(|question| !answered_ids.contains(&question.question_id))
        .unwrap_or(active.questions.len());

    let is_complete = active.current_index >= active.questions.len();
    let (current_question, summary, next_action) = if is_complete {
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

    let response = MarkStudyEntryMasteredResponse {
        entry_source_id: request.entry_source_id,
        entry_id,
        pruned_question_count,
        is_complete,
        current_question,
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
        answered_questions: answered_questions(active),
    };

    if is_complete {
        if let Some(summary) = response.summary.as_ref() {
            if let Some(next_action) = response.next_action.as_ref() {
                if let Err(error) = persistence::study_repo::save_completed_session(
                    conn,
                    &active.session,
                    summary,
                    &active.results,
                    next_action,
                ) {
                    eprintln!("study persistence failed but mastered session will still complete: {error}");
                }
            }
        }
        clear_persisted_session(conn, &active.session.mode)?;
        guard.remove(&mode_key);
    } else {
        persist_active_session(conn, active)?;
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
        answered_questions: answered_questions(active),
    }
}

fn answered_questions(active: &ActiveSession) -> Vec<AnsweredStudyQuestion> {
    active
        .results
        .iter()
        .filter_map(|result| {
            let question_index = active.question_map.get(&result.question_id).copied()?;
            let question = active.questions.get(question_index)?.clone();
            Some(AnsweredStudyQuestion {
                question,
                result: result.clone(),
            })
        })
        .collect()
}

fn rebuild_question_map(active: &mut ActiveSession) {
    active.question_map.clear();
    for (index, question) in active.questions.iter().enumerate() {
        active
            .question_map
            .insert(question.question_id.clone(), index);
    }
}

fn snapshot_question_weights_stale(
    snapshot: &ActiveSessionSnapshot,
    question_type_weights: &[QuestionTypeWeight],
) -> bool {
    snapshot.question_type_weights != question_type_weights
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

fn snapshot_matches_request(
    snapshot: &ActiveSessionSnapshot,
    request: &StartSessionRequest,
    question_type_weights: &[QuestionTypeWeight],
) -> bool {
    if request.entry_source_ids.is_empty() && request.entry_payloads.is_empty() {
        return true;
    }
    if snapshot.question_type_weights != question_type_weights {
        return false;
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

fn active_session_matches_request(
    active: &ActiveSession,
    request: &StartSessionRequest,
    question_type_weights: &[QuestionTypeWeight],
) -> bool {
    if request.entry_source_ids.is_empty() && request.entry_payloads.is_empty() {
        return true;
    }
    if active.question_type_weights != question_type_weights {
        return false;
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

fn normalized_question_type_weights_for_session(
    mode: &word_storage_core::models::SessionMode,
    weights: &[QuestionTypeWeight],
) -> Vec<QuestionTypeWeight> {
    if matches!(
        mode,
        word_storage_core::models::SessionMode::NewWord
            | word_storage_core::models::SessionMode::RootAffix
    ) {
        return Vec::new();
    }

    let mut normalized = weights
        .iter()
        .filter(|weight| weight.weight > 0)
        .cloned()
        .collect::<Vec<_>>();
    normalized.sort_by(|left, right| {
        question_type_sort_key(&left.question_type)
            .cmp(&question_type_sort_key(&right.question_type))
            .then_with(|| left.weight.cmp(&right.weight))
    });
    normalized
}

fn question_type_sort_key(question_type: &word_storage_core::models::QuestionType) -> u8 {
    match question_type {
        word_storage_core::models::QuestionType::EnToCnChoice => 0,
        word_storage_core::models::QuestionType::ExampleToCnChoice => 1,
        word_storage_core::models::QuestionType::ExampleToCnChoiceNoTranslation => 2,
        word_storage_core::models::QuestionType::CnToEnChoice => 3,
        word_storage_core::models::QuestionType::EnToCnInput => 4,
        word_storage_core::models::QuestionType::WordSkeletonInput => 5,
        word_storage_core::models::QuestionType::GlossToRootInput => 6,
        word_storage_core::models::QuestionType::RootToGlossInput => 7,
    }
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
        question_type_weights: snapshot.question_type_weights,
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
        question_type_weights: active.question_type_weights.clone(),
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
        word_storage_core::models::SessionMode::NewWord => total_words * 4,
        _ => total_words,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clear_all_active_sessions, mark_study_entry_mastered, start_study_session,
        submit_study_answer,
    };
    use rusqlite::OptionalExtension;
    use word_storage_core::models::{
        AnswerOutcome, ChoiceOption, MarkStudyEntryMasteredRequest, QuestionType,
        QuestionTypeWeight, SessionMode, StartSessionEntryPayload, StartSessionMeaningPayload,
        StartSessionRequest, StudyQuestion, SubmitAnswerRequest,
    };

    fn test_entry(source_id: &str, word: &str, meaning: &str) -> StartSessionEntryPayload {
        StartSessionEntryPayload {
            source_id: source_id.to_string(),
            word: word.to_string(),
            part_of_speech: Some("n".to_string()),
            frequency: 1.0,
            phonetic_us: None,
            phonetic_uk: None,
            meaning_details: vec![StartSessionMeaningPayload {
                pos: "n".to_string(),
                meaning_cn: meaning.to_string(),
                meaning_en: None,
            }],
            meanings: vec![meaning.to_string()],
            example_sentence: Some(format!("{word} example")),
            example_translation: Some(format!("{meaning} translation")),
            cn_choice_distractors: Vec::new(),
            en_choice_distractors: Vec::new(),
        }
    }

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
                    cn_choice_distractors: Vec::new(),
                    en_choice_distractors: Vec::new(),
                }],
                distractor_payloads: vec![
                    StartSessionEntryPayload {
                        source_id: "gesture".to_string(),
                        word: "gesture".to_string(),
                        part_of_speech: Some("vi".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "vi".to_string(),
                            meaning_cn: "做手势；用动作示意".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["做手势；用动作示意".to_string()],
                        example_sentence: Some("I gestured toward the boathouse.".to_string()),
                        example_translation: Some("我朝船屋做了个手势。".to_string()),
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                    StartSessionEntryPayload {
                        source_id: "delete".to_string(),
                        word: "delete".to_string(),
                        part_of_speech: Some("vi".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "vi".to_string(),
                            meaning_cn: "取消；删去；划掉；把...作废".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["取消；删去；划掉；把...作废".to_string()],
                        example_sentence: None,
                        example_translation: None,
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                ],
                question_type_weights: Vec::new(),
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
                    cn_choice_distractors: Vec::new(),
                    en_choice_distractors: Vec::new(),
                }],
                distractor_payloads: vec![StartSessionEntryPayload {
                    source_id: "delete".to_string(),
                    word: "delete".to_string(),
                    part_of_speech: Some("vi".to_string()),
                    frequency: 1.0,
                    phonetic_us: None,
                    phonetic_uk: None,
                    meaning_details: vec![StartSessionMeaningPayload {
                        pos: "vi".to_string(),
                        meaning_cn: "\u{53d6}\u{6d88}\u{ff1b}\u{5220}\u{53bb}\u{ff1b}\u{5212}\u{6389}\u{ff1b}\u{628a}...\u{4f5c}\u{5e9f}".to_string(),
                        meaning_en: None,
                    }],
                    meanings: vec!["\u{53d6}\u{6d88}\u{ff1b}\u{5220}\u{53bb}\u{ff1b}\u{5212}\u{6389}\u{ff1b}\u{628a}...\u{4f5c}\u{5e9f}".to_string()],
                    example_sentence: None,
                    example_translation: None,
                    cn_choice_distractors: Vec::new(),
                    en_choice_distractors: Vec::new(),
                }],
                question_type_weights: Vec::new(),
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
                    cn_choice_distractors: Vec::new(),
                    en_choice_distractors: Vec::new(),
                }],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
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
                    cn_choice_distractors: Vec::new(),
                    en_choice_distractors: Vec::new(),
                }],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("start second session from changed sources");

        assert_eq!(first.current_question.entry_source_id, "book_one_word");
        assert_eq!(second.current_question.entry_source_id, "book_two_word");
        assert_eq!(second.session.wordbook_id, Some(2));
        assert_ne!(first.session.session_id, second.session.session_id);
    }

    #[test]
    fn start_session_rebuilds_in_memory_session_when_question_weights_changed() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let payload = StartSessionEntryPayload {
            source_id: "shared_word".to_string(),
            word: "fridge".to_string(),
            part_of_speech: Some("n".to_string()),
            frequency: 1.0,
            phonetic_us: None,
            phonetic_uk: None,
            meaning_details: vec![StartSessionMeaningPayload {
                pos: "n".to_string(),
                meaning_cn: "冰箱".to_string(),
                meaning_en: None,
            }],
            meanings: vec!["冰箱".to_string()],
            example_sentence: Some("Put the milk in the fridge.".to_string()),
            example_translation: Some("把牛奶放进冰箱。".to_string()),
            cn_choice_distractors: Vec::new(),
            en_choice_distractors: Vec::new(),
        };

        let first = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: Some(1),
                entry_source_ids: vec!["shared_word".to_string()],
                entry_payloads: vec![payload.clone()],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnChoice,
                    weight: 100,
                }],
            },
        )
        .expect("start first weighted session");

        let second = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: Some(1),
                entry_source_ids: vec!["shared_word".to_string()],
                entry_payloads: vec![payload],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::WordSkeletonInput,
                    weight: 100,
                }],
            },
        )
        .expect("start session after plan weights changed");

        assert_eq!(
            first.current_question.question_type,
            QuestionType::EnToCnChoice
        );
        assert_eq!(
            second.current_question.question_type,
            QuestionType::WordSkeletonInput
        );
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
            "questionEngineVersion": super::QUESTION_ENGINE_VERSION,
            "session": {
                "sessionId": "sess_resume",
                "mode": "newWord",
                "totalWords": 8,
                "wordbookId": null,
                "startedAt": started_at
            },
            "questions": questions,
            "results": [],
            "currentIndex": 8,
            "questionTypeWeights": [{
                "questionType": "enToCnChoice",
                "weight": 100
            }]
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
                question_type_weights: Vec::new(),
            },
        )
        .expect("empty start request should restore saved progress");

        assert_eq!(response.session.session_id, "sess_resume");
        assert_eq!(response.progress.current, 9);
        assert_eq!(response.progress.total, 32);
        assert_eq!(response.current_question.question_id, "sess_resume_8");
    }

    #[test]
    fn submit_answer_rejects_non_current_question_id() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["condemn".to_string(), "spoil".to_string()],
                entry_payloads: vec![
                    StartSessionEntryPayload {
                        source_id: "condemn".to_string(),
                        word: "condemn".to_string(),
                        part_of_speech: Some("vt".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "vt".to_string(),
                            meaning_cn: "condemn meaning".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["condemn meaning".to_string()],
                        example_sentence: None,
                        example_translation: None,
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                    StartSessionEntryPayload {
                        source_id: "spoil".to_string(),
                        word: "spoil".to_string(),
                        part_of_speech: Some("vt".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "vt".to_string(),
                            meaning_cn: "spoil meaning".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["spoil meaning".to_string()],
                        example_sentence: None,
                        example_translation: None,
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                ],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("start session");

        let second_question = {
            let guard = super::lock_sessions();
            let active = guard
                .values()
                .find(|active| active.session.session_id == start.session.session_id)
                .expect("active session exists");
            active.questions[1].clone()
        };
        let second_correct_label = second_question
            .correct_choice_label
            .clone()
            .expect("second choice has correct label");

        let error = submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: second_question.question_id.clone(),
                response: second_correct_label,
                response_time_ms: 100,
            },
        )
        .expect_err("non-current question submit should be rejected");

        assert!(matches!(error, super::StudyError::NoActiveSession));
    }

    #[test]
    fn answered_question_history_survives_submit_and_resume() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["alpha".to_string(), "beta".to_string()],
                entry_payloads: vec![
                    StartSessionEntryPayload {
                        source_id: "alpha".to_string(),
                        word: "alpha".to_string(),
                        part_of_speech: Some("n".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "n".to_string(),
                            meaning_cn: "alpha meaning".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["alpha meaning".to_string()],
                        example_sentence: None,
                        example_translation: None,
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                    StartSessionEntryPayload {
                        source_id: "beta".to_string(),
                        word: "beta".to_string(),
                        part_of_speech: Some("n".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "n".to_string(),
                            meaning_cn: "beta meaning".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["beta meaning".to_string()],
                        example_sentence: None,
                        example_translation: None,
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                ],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("start session");

        assert!(start.answered_questions.is_empty());
        let answered_question_id = start.current_question.question_id.clone();
        let answered_entry_id = start.current_question.entry_source_id.clone();
        let correct_response = start
            .current_question
            .correct_choice_label
            .clone()
            .unwrap_or_else(|| start.current_question.accepted_meanings[0].clone());

        let submit = submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: answered_question_id.clone(),
                response: correct_response,
                response_time_ms: 123,
            },
        )
        .expect("submit first answer");

        assert_eq!(submit.answered_questions.len(), 1);
        assert_eq!(
            submit.answered_questions[0].question.question_id,
            answered_question_id
        );
        assert_eq!(
            submit.answered_questions[0].question.entry_source_id,
            answered_entry_id
        );
        assert_eq!(
            submit.answered_questions[0].result.question_id,
            answered_question_id
        );

        clear_all_active_sessions();
        let resumed = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("resume session");

        assert_eq!(resumed.answered_questions.len(), 1);
        assert_eq!(
            resumed.answered_questions[0].question.question_id,
            answered_question_id
        );
        assert_eq!(
            resumed.answered_questions[0].result.question_id,
            answered_question_id
        );
    }

    #[test]
    fn final_submit_persists_completed_session_and_clears_resume_snapshot() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["alpha".to_string()],
                entry_payloads: vec![StartSessionEntryPayload {
                    source_id: "alpha".to_string(),
                    word: "alpha".to_string(),
                    part_of_speech: Some("n".to_string()),
                    frequency: 1.0,
                    phonetic_us: None,
                    phonetic_uk: None,
                    meaning_details: vec![StartSessionMeaningPayload {
                        pos: "n".to_string(),
                        meaning_cn: "alpha meaning".to_string(),
                        meaning_en: None,
                    }],
                    meanings: vec!["alpha meaning".to_string()],
                    example_sentence: None,
                    example_translation: None,
                    cn_choice_distractors: Vec::new(),
                    en_choice_distractors: Vec::new(),
                }],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: word_storage_core::models::QuestionType::EnToCnInput,
                    weight: 100,
                }],
            },
        )
        .expect("start session");

        assert_eq!(start.progress.total, 1);
        let submit = submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: start.current_question.question_id.clone(),
                response: "wrong".to_string(),
                response_time_ms: 10,
            },
        )
        .expect("submit final answer");

        assert!(submit.is_complete);
        assert_eq!(submit.progress.current, 1);
        assert_eq!(submit.progress.total, 1);
        assert_eq!(submit.result.correct_answer, "alpha meaning");

        let completed_at: Option<String> = conn
            .query_row(
                "SELECT completed_at FROM study_sessions WHERE session_id = ?1",
                [&start.session.session_id],
                |row| row.get(0),
            )
            .expect("completed session row");
        assert!(completed_at.is_some());

        let snapshot_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM app_settings WHERE key = ?1",
                ["active_study_session_mixedTest"],
                |row| row.get(0),
            )
            .expect("snapshot count");
        assert_eq!(snapshot_count, 0);

        clear_all_active_sessions();
        let error = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect_err("completed submit must not leave a resumable snapshot");
        assert!(matches!(error, super::StudyError::NotEnoughWords));
    }

    #[test]
    fn submit_choice_returns_wrong_label_and_unique_correct_answer() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["gesture".to_string(), "delete".to_string()],
                entry_payloads: vec![
                    StartSessionEntryPayload {
                        source_id: "gesture".to_string(),
                        word: "gesture".to_string(),
                        part_of_speech: Some("vi".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "vi".to_string(),
                            meaning_cn: "做手势；用动作示意".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["做手势；用动作示意".to_string()],
                        example_sentence: Some("I gestured toward the boathouse.".to_string()),
                        example_translation: Some("我朝船屋做了个手势。".to_string()),
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                    StartSessionEntryPayload {
                        source_id: "delete".to_string(),
                        word: "delete".to_string(),
                        part_of_speech: Some("vi".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "vi".to_string(),
                            meaning_cn: "取消；删去；划掉；把...作废".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["取消；删去；划掉；把...作废".to_string()],
                        example_sentence: None,
                        example_translation: None,
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                ],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: word_storage_core::models::QuestionType::EnToCnChoice,
                    weight: 100,
                }],
            },
        )
        .expect("start session");

        assert_eq!(start.current_question.word, "gesture");
        let correct_label = start
            .current_question
            .choices
            .as_ref()
            .and_then(|choices| {
                choices
                    .iter()
                    .find(|choice| choice.text == "做手势；用动作示意")
            })
            .map(|choice| choice.label.clone())
            .expect("correct gesture choice exists");
        let wrong_label = start
            .current_question
            .choices
            .as_ref()
            .and_then(|choices| {
                choices
                    .iter()
                    .find(|choice| choice.text == "取消；删去；划掉；把...作废")
            })
            .map(|choice| choice.label.clone())
            .unwrap_or_else(|| "B".to_string());
        assert_ne!(wrong_label, correct_label);

        let submit = submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: start.current_question.question_id.clone(),
                response: wrong_label.clone(),
                response_time_ms: 100,
            },
        )
        .expect("submit wrong answer");

        assert_eq!(submit.result.user_response, wrong_label);
        assert_eq!(
            submit.result.outcome,
            word_storage_core::models::AnswerOutcome::Incorrect
        );
        assert_eq!(submit.result.correct_answer, "做手势；用动作示意");
        assert_eq!(submit.answered_questions.len(), 1);
        assert_eq!(
            submit.answered_questions[0].result.user_response,
            wrong_label
        );
        assert_eq!(
            submit.answered_questions[0].result.correct_answer,
            "做手势；用动作示意"
        );
    }

    #[test]
    fn non_a_choice_label_survives_start_submit_history_and_resume() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let payloads = vec![
            test_entry("alpha", "alpha", "alpha meaning"),
            test_entry("beta", "beta", "beta meaning"),
            test_entry("gamma", "gamma", "gamma meaning"),
            test_entry("delta", "delta", "delta meaning"),
        ];

        let mut current_question = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: payloads
                    .iter()
                    .map(|payload| payload.source_id.clone())
                    .collect(),
                entry_payloads: payloads.clone(),
                distractor_payloads: payloads,
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnChoice,
                    weight: 100,
                }],
            },
        )
        .expect("start choice session")
        .current_question;

        for _ in 0..4 {
            let correct_label = current_question
                .correct_choice_label
                .clone()
                .expect("choice question has correct label");
            let correct_text = current_question
                .choices
                .as_ref()
                .and_then(|choices| {
                    choices
                        .iter()
                        .find(|choice| choice.label == correct_label)
                        .map(|choice| choice.text.clone())
                })
                .expect("correct choice text exists");

            if correct_label != "A" {
                let submit = submit_study_answer(
                    &conn,
                    SubmitAnswerRequest {
                        question_id: current_question.question_id.clone(),
                        response: "A".to_string(),
                        response_time_ms: 100,
                    },
                )
                .expect("submit wrong A answer");

                assert_eq!(submit.result.user_response, "A");
                assert_eq!(submit.result.outcome, AnswerOutcome::Incorrect);
                assert_eq!(submit.result.correct_answer, correct_text);
                let submitted_history = submit
                    .answered_questions
                    .last()
                    .expect("submitted question appears in history");
                assert_eq!(
                    submitted_history.question.correct_choice_label.as_deref(),
                    Some(correct_label.as_str())
                );

                clear_all_active_sessions();
                let resumed = start_study_session(
                    &conn,
                    StartSessionRequest {
                        mode: SessionMode::MixedTest,
                        wordbook_id: None,
                        entry_source_ids: Vec::new(),
                        entry_payloads: Vec::new(),
                        distractor_payloads: Vec::new(),
                        question_type_weights: vec![QuestionTypeWeight {
                            question_type: QuestionType::EnToCnChoice,
                            weight: 100,
                        }],
                    },
                )
                .expect("resume active choice session");

                let resumed_history = resumed
                    .answered_questions
                    .last()
                    .expect("submitted question survives resume history");
                assert_eq!(
                    resumed_history.question.correct_choice_label.as_deref(),
                    Some(correct_label.as_str())
                );
                assert_eq!(resumed_history.result.user_response, "A");
                assert_eq!(resumed_history.result.outcome, AnswerOutcome::Incorrect);
                return;
            }

            let submit = submit_study_answer(
                &conn,
                SubmitAnswerRequest {
                    question_id: current_question.question_id.clone(),
                    response: correct_label,
                    response_time_ms: 100,
                },
            )
            .expect("submit correct answer to reach non-A question");

            current_question = submit
                .current_question
                .expect("session should contain a later non-A choice question");
        }

        panic!("choice generator should produce at least one non-A correct label");
    }

    #[test]
    fn submitting_non_a_correct_choice_is_correct_not_a_fallback() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let payloads = vec![
            test_entry("alpha", "alpha", "alpha meaning"),
            test_entry("beta", "beta", "beta meaning"),
            test_entry("gamma", "gamma", "gamma meaning"),
            test_entry("delta", "delta", "delta meaning"),
        ];

        let mut current_question = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: payloads
                    .iter()
                    .map(|payload| payload.source_id.clone())
                    .collect(),
                entry_payloads: payloads.clone(),
                distractor_payloads: payloads,
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnChoice,
                    weight: 100,
                }],
            },
        )
        .expect("start choice session")
        .current_question;

        for _ in 0..4 {
            let correct_label = current_question
                .correct_choice_label
                .clone()
                .expect("choice question has correct label");

            if correct_label != "A" {
                let submit = submit_study_answer(
                    &conn,
                    SubmitAnswerRequest {
                        question_id: current_question.question_id.clone(),
                        response: correct_label.clone(),
                        response_time_ms: 100,
                    },
                )
                .expect("submit non-A correct answer");

                assert_eq!(submit.result.user_response, correct_label);
                assert_eq!(submit.result.outcome, AnswerOutcome::Correct);
                return;
            }

            let submit = submit_study_answer(
                &conn,
                SubmitAnswerRequest {
                    question_id: current_question.question_id.clone(),
                    response: correct_label,
                    response_time_ms: 100,
                },
            )
            .expect("submit A answer to reach non-A question");

            current_question = submit
                .current_question
                .expect("session should contain a later non-A choice question");
        }

        panic!("choice generator should produce at least one non-A correct label");
    }

    #[test]
    fn stale_engine_snapshot_is_deleted_on_empty_resume_attempt() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let snapshot = serde_json::json!({
            "questionEngineVersion": 7,
            "session": {
                "sessionId": "sess_stale_choice_labels",
                "mode": "mixedTest",
                "totalWords": 1,
                "wordbookId": null,
                "startedAt": chrono::Utc::now().to_rfc3339()
            },
            "questions": [serde_json::to_value(StudyQuestion {
                question_id: "sess_stale_choice_labels_0".to_string(),
                question_type: QuestionType::EnToCnChoice,
                entry_source_id: "alpha".to_string(),
                word: "alpha".to_string(),
                part_of_speech: Some("n".to_string()),
                phonetic_us: None,
                phonetic_uk: None,
                prompt: "alpha".to_string(),
                accepted_meanings: vec!["alpha meaning".to_string()],
                example_sentence: None,
                example_translation: None,
                choices: Some(vec![ChoiceOption {
                    label: "A".to_string(),
                    text: "alpha meaning".to_string(),
                }]),
                correct_choice_label: Some("A".to_string()),
                question_index: 0,
                total_questions: 1,
            }).expect("serialize question")],
            "results": [],
            "currentIndex": 0,
            "questionTypeWeights": [{
                "questionType": "enToCnChoice",
                "weight": 100
            }]
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_mixedTest", snapshot.to_string()),
        )
        .expect("insert stale snapshot");

        let error = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnChoice,
                    weight: 100,
                }],
            },
        )
        .expect_err("empty resume cannot build a new session after deleting stale snapshot");

        assert!(matches!(error, super::StudyError::NotEnoughWords));
        let remaining: Option<String> = conn
            .query_row(
                "SELECT value_json FROM app_settings WHERE key = ?1",
                ["active_study_session_mixedTest"],
                |row| row.get(0),
            )
            .optional()
            .expect("query stale snapshot");
        assert!(remaining.is_none());
    }

    #[test]
    fn mastered_entry_prunes_unanswered_new_word_group() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::NewWord,
                wordbook_id: None,
                entry_source_ids: vec!["alpha".to_string(), "beta".to_string()],
                entry_payloads: vec![
                    StartSessionEntryPayload {
                        source_id: "alpha".to_string(),
                        word: "alpha".to_string(),
                        part_of_speech: Some("n".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "n".to_string(),
                            meaning_cn: "alpha meaning".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["alpha meaning".to_string()],
                        example_sentence: Some("alpha example".to_string()),
                        example_translation: Some("alpha translation".to_string()),
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                    StartSessionEntryPayload {
                        source_id: "beta".to_string(),
                        word: "beta".to_string(),
                        part_of_speech: Some("n".to_string()),
                        frequency: 1.0,
                        phonetic_us: None,
                        phonetic_uk: None,
                        meaning_details: vec![StartSessionMeaningPayload {
                            pos: "n".to_string(),
                            meaning_cn: "beta meaning".to_string(),
                            meaning_en: None,
                        }],
                        meanings: vec!["beta meaning".to_string()],
                        example_sentence: Some("beta example".to_string()),
                        example_translation: Some("beta translation".to_string()),
                        cn_choice_distractors: Vec::new(),
                        en_choice_distractors: Vec::new(),
                    },
                ],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("start session");

        assert_eq!(start.progress.total, 8);
        let pruned = mark_study_entry_mastered(
            &conn,
            MarkStudyEntryMasteredRequest {
                entry_source_id: start.current_question.entry_source_id.clone(),
                reason: "mastered".to_string(),
            },
        )
        .expect("mark mastered");

        assert_eq!(pruned.pruned_question_count, 4);
        assert!(!pruned.is_complete);
        assert_eq!(pruned.progress.total, 4);
        assert_ne!(
            pruned.current_question.unwrap().entry_source_id,
            start.current_question.entry_source_id
        );
    }

    #[test]
    fn mastered_entry_completes_when_last_pending_word_is_pruned() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["alpha".to_string()],
                entry_payloads: vec![StartSessionEntryPayload {
                    source_id: "alpha".to_string(),
                    word: "alpha".to_string(),
                    part_of_speech: Some("n".to_string()),
                    frequency: 1.0,
                    phonetic_us: None,
                    phonetic_uk: None,
                    meaning_details: vec![StartSessionMeaningPayload {
                        pos: "n".to_string(),
                        meaning_cn: "alpha meaning".to_string(),
                        meaning_en: None,
                    }],
                    meanings: vec!["alpha meaning".to_string()],
                    example_sentence: None,
                    example_translation: None,
                    cn_choice_distractors: Vec::new(),
                    en_choice_distractors: Vec::new(),
                }],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("start session");

        let response = mark_study_entry_mastered(
            &conn,
            MarkStudyEntryMasteredRequest {
                entry_source_id: start.current_question.entry_source_id,
                reason: "mastered".to_string(),
            },
        )
        .expect("mark mastered");

        assert!(response.is_complete);
        assert_eq!(response.pruned_question_count, 1);
        assert!(response.current_question.is_none());
        assert!(response.summary.is_some());
        assert!(
            word_storage_core::persistence::mastered_entry_repo::is_mastered_source_id(
                &conn, "alpha"
            )
            .expect("query mastered")
        );
    }
}
