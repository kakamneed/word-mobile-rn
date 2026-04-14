//! Study session facade for learning loop.

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use rusqlite::Connection;

use word_storage_core::models::{
    ChoiceOption, SessionMode, SessionProgress, SessionSummary, StartSessionRequest,
    StartSessionResponse, StudyQuestion, StudyResult, StudySession, SubmitAnswerRequest,
    SubmitAnswerResponse, CompleteSessionResponse,
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

// Global in-memory session storage
static ACTIVE_SESSION: Lazy<Mutex<Option<ActiveSession>>> = Lazy::new(|| Mutex::new(None));

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
    _conn: &Connection,
    request: StartSessionRequest,
) -> Result<StartSessionResponse, StudyError> {
    // Check for existing session
    {
        let guard = ACTIVE_SESSION.lock().map_err(|_| StudyError::NoActiveSession)?;
        if guard.is_some() {
            return Err(StudyError::SessionAlreadyActive);
        }
    }

    let mode = request.mode;
    let total_words = request.entry_source_ids.len() as u32;

    if total_words == 0 {
        return Err(StudyError::NotEnoughWords);
    }

    // Create words for question builder
    let words: Vec<WordForQuestion> = request
        .entry_source_ids
        .iter()
        .map(|id| WordForQuestion {
            source_id: id.clone(),
            word: id.clone(), // Placeholder - would fetch from DB
            meanings: vec![],
            examples: vec![],
        })
        .collect();

    let session_id = format!("sess_{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
    let started_at = chrono::Utc::now().to_rfc3339();

    // Generate questions
    let questions = QuestionBuilder::build_session_questions(&mode, &words, &words, &session_id);

    if questions.is_empty() {
        return Err(StudyError::NotEnoughWords);
    }

    let session = StudySession {
        session_id: session_id.clone(),
        mode,
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
    {
        let mut guard = ACTIVE_SESSION.lock().map_err(|_| StudyError::NoActiveSession)?;
        *guard = Some(ActiveSession {
            session,
            questions: questions.clone(),
            results: Vec::new(),
            current_index: 0,
            question_map,
        });
    }

    let first_question = questions[0].clone();

    Ok(StartSessionResponse {
        session: StudySession {
            session_id: session_id.clone(),
            mode: request.mode,
            total_words,
            wordbook_id: request.wordbook_id,
            started_at: chrono::Utc::now().to_rfc3339(),
        },
        current_question: first_question,
        progress: SessionProgress {
            current: 1,
            total: questions.len() as u32,
        },
    })
}

/// Submit an answer for the current question.
pub fn submit_study_answer(
    _conn: &Connection,
    request: SubmitAnswerRequest,
) -> Result<SubmitAnswerResponse, StudyError> {
    let mut guard = ACTIVE_SESSION.lock().map_err(|_| StudyError::NoActiveSession)?;
    let active = guard.as_mut().ok_or(StudyError::NoActiveSession)?;

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
        let summary = SessionSummaryService::build_summary(
            &active.session,
            &active.results,
            &completed_at,
        );
        let next_action = SessionSummaryService::next_action(&summary, &active.session.mode);
        (None, Some(summary), Some(next_action))
    } else {
        (
            Some(active.questions[active.current_index].clone()),
            None,
            None,
        )
    };

    Ok(SubmitAnswerResponse {
        result,
        is_complete,
        current_question: next_question,
        summary,
        next_action,
        progress: SessionProgress {
            current: active.current_index as u32 + 1,
            total: active.questions.len() as u32,
        },
    })
}

/// Complete the active session and persist results.
pub fn complete_study_session(
    conn: &Connection,
) -> Result<CompleteSessionResponse, StudyError> {
    let mut guard = ACTIVE_SESSION.lock().map_err(|_| StudyError::NoActiveSession)?;
    let active = guard.as_mut().ok_or(StudyError::NoActiveSession)?;

    let completed_at = chrono::Utc::now().to_rfc3339();
    let summary = SessionSummaryService::build_summary(
        &active.session,
        &active.results,
        &completed_at,
    );
    let next_action = SessionSummaryService::next_action(&summary, &active.session.mode);

    // Persist to database
    persistence::study_repo::save_completed_session(
        conn,
        &active.session,
        &summary,
        &active.results,
        &next_action,
    )
    .map_err(|e| StudyError::Storage(e.to_string()))?;

    // Clear active session
    *guard = None;

    Ok(CompleteSessionResponse {
        summary,
        next_action,
    })
}

/// Cancel the active session without persisting.
pub fn cancel_study_session() -> Result<(), StudyError> {
    let mut guard = ACTIVE_SESSION.lock().map_err(|_| StudyError::NoActiveSession)?;
    *guard = None;
    Ok(())
}
