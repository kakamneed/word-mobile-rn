//! Study session facade for learning loop.

use std::collections::HashMap;
use std::collections::HashSet;
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
#[derive(Clone)]
struct ActiveSession {
    session: StudySession,
    questions: Vec<StudyQuestion>,
    results: Vec<StudyResult>,
    current_index: usize,
    question_map: HashMap<String, usize>,
    question_type_weights: Vec<QuestionTypeWeight>,
    entry_source_ids: Vec<String>,
    entry_payloads: Vec<StartSessionEntryPayload>,
    distractor_payloads: Vec<StartSessionEntryPayload>,
    question_plan_mutated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapshotRejectionReason {
    EngineVersion,
    Shape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveSessionSnapshotV1 {
    #[serde(default = "default_active_snapshot_schema_version_v1")]
    schema_version: i64,
    #[serde(default = "default_question_engine_version")]
    question_engine_version: i64,
    session: StudySession,
    questions: Vec<StudyQuestion>,
    results: Vec<StudyResult>,
    current_index: usize,
    #[serde(default)]
    question_type_weights: Vec<QuestionTypeWeight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveSessionSnapshotV2 {
    schema_version: i64,
    question_engine_version: i64,
    #[serde(default)]
    vocabulary_version: Option<String>,
    session: StudySession,
    study_date: String,
    entry_source_ids: Vec<String>,
    #[serde(default)]
    entry_payloads: Vec<StartSessionEntryPayload>,
    #[serde(default)]
    distractor_payloads: Vec<StartSessionEntryPayload>,
    #[serde(default)]
    question_plan_signature: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    questions: Vec<StudyQuestion>,
    current_index: usize,
    results: Vec<StudyResult>,
    #[serde(default)]
    question_type_weights: Vec<QuestionTypeWeight>,
}

enum LoadedActiveSessionSnapshot {
    V1(ActiveSessionSnapshotV1),
    V2(ActiveSessionSnapshotV2),
}

// Global in-memory session storage
static ACTIVE_SESSIONS: Lazy<Mutex<HashMap<String, ActiveSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static RECENT_COMPLETED_SESSIONS: Lazy<Mutex<HashMap<String, ActiveSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));
static STUDY_DIAGNOSTICS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

const QUESTION_ENGINE_VERSION: i64 = 10;
const ACTIVE_SNAPSHOT_SCHEMA_VERSION_V1: i64 = 1;
const ACTIVE_SNAPSHOT_SCHEMA_VERSION_V2: i64 = 2;
const ANSWERED_FEED_WINDOW: usize = 20;

fn default_active_snapshot_schema_version_v1() -> i64 {
    ACTIVE_SNAPSHOT_SCHEMA_VERSION_V1
}

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
    RECENT_COMPLETED_SESSIONS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    STUDY_DIAGNOSTICS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
}

/// Return recent study self-repair diagnostics for debugging bridge/session state.
pub fn recent_study_diagnostics() -> Vec<String> {
    STUDY_DIAGNOSTICS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

fn record_study_diagnostic(message: impl Into<String>) {
    let mut diagnostics = STUDY_DIAGNOSTICS.lock().unwrap_or_else(|e| e.into_inner());
    diagnostics.push(message.into());
    let overflow = diagnostics.len().saturating_sub(50);
    if overflow > 0 {
        diagnostics.drain(0..overflow);
    }
}

/// Lock ACTIVE_SESSIONS, recovering from mutex poison.
fn lock_sessions() -> std::sync::MutexGuard<'static, HashMap<String, ActiveSession>> {
    ACTIVE_SESSIONS.lock().unwrap_or_else(|e| e.into_inner())
}

fn lock_recent_completed_sessions() -> std::sync::MutexGuard<'static, HashMap<String, ActiveSession>>
{
    RECENT_COMPLETED_SESSIONS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
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
    let usable_entry_payloads = valid_entry_payloads(&request.entry_payloads);
    let usable_distractor_payloads = valid_entry_payloads(&request.distractor_payloads);
    let usable_entry_source_ids = valid_entry_source_ids(&request.entry_source_ids);
    let total_words = if request.entry_payloads.is_empty() {
        usable_entry_source_ids.len() as u32
    } else {
        usable_entry_payloads.len() as u32
    };
    let question_type_weights =
        normalized_question_type_weights_for_session(&mode, &request.question_type_weights);
    let mut cleared_stale_persisted_snapshot = false;

    if !request.entry_source_ids.is_empty() || !request.entry_payloads.is_empty() {
        let mut guard = lock_sessions();
        if guard
            .get(&mode_key)
            .map(|active| active_session_snapshot_stale(active, &mode))
            .unwrap_or(false)
        {
            guard.remove(&mode_key);
            drop(guard);
            clear_persisted_session(conn, &mode)?;
        } else if let Some(active) = guard.get(&mode_key) {
            if active.current_index < active.questions.len()
                && active_session_matches_request(active, &request, &question_type_weights)
            {
                return Ok(build_start_response(active));
            }
        }
    }

    if let Some(loaded_snapshot) = load_persisted_session(conn, &mode)? {
        let snapshot = match loaded_snapshot {
            LoadedActiveSessionSnapshot::V1(snapshot) => snapshot,
            LoadedActiveSessionSnapshot::V2(snapshot) => {
                let request_is_empty =
                    request.entry_source_ids.is_empty() && request.entry_payloads.is_empty();
                if !request_is_empty
                    && !v2_snapshot_matches_request(&snapshot, &request, &question_type_weights)
                {
                    record_study_diagnostic(format!(
                        "cleared mismatched v2 active snapshot for mode {}",
                        mode_storage_key(&mode)
                    ));
                    clear_persisted_session(conn, &mode)?;
                    lock_sessions().remove(&mode_key);
                    return start_study_session(conn, request);
                }
                if let Some(active) = active_session_from_v2_snapshot(snapshot, &mode) {
                    let response = build_start_response(&active);
                    let mut guard = lock_sessions();
                    guard.insert(mode_key, active);
                    return Ok(response);
                }
                record_study_diagnostic(format!(
                    "cleared invalid v2 active snapshot for mode {}",
                    mode_storage_key(&mode)
                ));
                clear_persisted_session(conn, &mode)?;
                if !request_is_empty {
                    lock_sessions().remove(&mode_key);
                    return start_study_session(conn, request);
                }
                return Err(StudyError::NotEnoughWords);
            }
        };
        let expected_questions =
            if request.entry_source_ids.is_empty() && request.entry_payloads.is_empty() {
                snapshot.questions.len()
            } else {
                expected_question_count(&mode, total_words as usize)
            };
        let snapshot_rejection = snapshot_rejection_reason(&snapshot, &mode, expected_questions);
        if snapshot_rejection.is_none()
            && snapshot_matches_request(&snapshot, &request, &question_type_weights)
        {
            let active = active_session_from_snapshot(snapshot);
            let response = build_start_response(&active);
            let mut guard = lock_sessions();
            guard.insert(mode_key, active);
            return Ok(response);
        }
        if matches!(
            snapshot_rejection,
            Some(SnapshotRejectionReason::EngineVersion | SnapshotRejectionReason::Shape)
        ) {
            record_study_diagnostic(format!(
                "cleared invalid v1 active snapshot for mode {}: {:?}",
                mode_storage_key(&mode),
                snapshot_rejection.unwrap()
            ));
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
        let mut guard = lock_sessions();
        if guard
            .get(&mode_key)
            .map(|active| active_session_snapshot_stale(active, &mode))
            .unwrap_or(false)
        {
            guard.remove(&mode_key);
            drop(guard);
            clear_persisted_session(conn, &mode)?;
        } else if let Some(active) = guard.get(&mode_key) {
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
        payloads_to_words(&usable_entry_payloads)
    } else {
        usable_entry_source_ids
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
        payloads_to_words(&usable_distractor_payloads)
    } else {
        Vec::new()
    };

    let now = chrono::Utc::now();
    let session_id = format!("sess_{}", now.format("%Y%m%d%H%M%S%f"));
    let started_at = now.to_rfc3339();

    // Generate questions
    let mut questions = QuestionBuilder::build_session_questions(
        &mode,
        &words,
        &distractors,
        &session_id,
        &request.question_type_weights,
    );

    if questions.is_empty() {
        return Err(StudyError::NotEnoughWords);
    }
    questions = repair_study_questions(questions);
    questions = valid_study_questions(questions);
    normalize_question_indexes(&mut questions);

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
    let active_entry_source_ids = if usable_entry_source_ids.is_empty() {
        usable_entry_payloads
            .iter()
            .map(|payload| payload.source_id.clone())
            .collect()
    } else {
        usable_entry_source_ids.clone()
    };

    let active = ActiveSession {
        session,
        questions: questions.clone(),
        results: Vec::new(),
        current_index: 0,
        question_map,
        question_type_weights,
        entry_source_ids: active_entry_source_ids,
        entry_payloads: usable_entry_payloads.clone(),
        distractor_payloads: usable_distractor_payloads.clone(),
        question_plan_mutated: false,
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
            cn_choice_distractors: valid_choice_distractors(&payload.cn_choice_distractors),
            en_choice_distractors: valid_choice_distractors(&payload.en_choice_distractors),
        })
        .collect()
}

fn valid_entry_payloads(payloads: &[StartSessionEntryPayload]) -> Vec<StartSessionEntryPayload> {
    payloads
        .iter()
        .filter_map(|payload| {
            if valid_entry_payload(payload) {
                Some(payload.clone())
            } else {
                record_study_diagnostic(format!(
                    "skipped invalid study entry payload: {}",
                    payload.source_id
                ));
                None
            }
        })
        .collect()
}

fn valid_entry_payload(payload: &StartSessionEntryPayload) -> bool {
    has_display_content(&payload.source_id)
        && has_study_word_content(&payload.word)
        && payload_accepted_meanings(payload)
            .iter()
            .any(|meaning| has_display_content(meaning))
}

fn payload_accepted_meanings(payload: &StartSessionEntryPayload) -> Vec<String> {
    if payload.meaning_details.is_empty() {
        payload.meanings.clone()
    } else {
        payload
            .meaning_details
            .iter()
            .map(|meaning| meaning.meaning_cn.clone())
            .collect()
    }
}

fn valid_entry_source_ids(source_ids: &[String]) -> Vec<String> {
    source_ids
        .iter()
        .filter(|source_id| has_display_content(source_id))
        .cloned()
        .collect()
}

fn valid_choice_distractors(distractors: &[String]) -> Vec<String> {
    distractors
        .iter()
        .filter(|distractor| has_display_content(distractor))
        .cloned()
        .collect()
}

fn valid_study_question(question: &StudyQuestion) -> bool {
    if !has_display_content(&question.entry_source_id)
        || !has_study_word_content(&question.word)
        || !has_display_content(&question.prompt)
        || !question
            .accepted_meanings
            .iter()
            .any(|meaning| has_display_content(meaning))
    {
        return false;
    }

    if question.question_type.is_choice_type() {
        let Some(choices) = question.choices.as_ref() else {
            return false;
        };
        if choices.len() != 4 {
            return false;
        }
        let Some(correct_choice_label) = question.correct_choice_label.as_ref() else {
            return false;
        };
        if !has_display_content(correct_choice_label) {
            return false;
        }
        let mut labels = HashSet::new();
        let mut choice_texts = HashSet::new();
        let mut has_correct_choice = false;
        let mut correct_choice_text: Option<&str> = None;
        for choice in choices {
            if !has_display_content(&choice.label) || !has_display_content(&choice.text) {
                return false;
            }
            if !labels.insert(choice.label.clone()) {
                return false;
            }
            if !choice_texts.insert(choice.text.clone()) {
                return false;
            }
            if choice.label == *correct_choice_label {
                has_correct_choice = true;
                correct_choice_text = Some(choice.text.as_str());
            }
        }
        if !has_correct_choice {
            return false;
        }
        if !correct_choice_matches_question(question, correct_choice_text.unwrap_or_default()) {
            return false;
        }
    }

    true
}

fn repair_study_questions(questions: Vec<StudyQuestion>) -> Vec<StudyQuestion> {
    questions
        .into_iter()
        .map(|question| {
            if choice_question_has_complete_options(&question) {
                question
            } else if question.question_type.is_choice_type() {
                downgrade_choice_question_to_input(question)
            } else {
                question
            }
        })
        .collect()
}

fn choice_question_has_complete_options(question: &StudyQuestion) -> bool {
    if !question.question_type.is_choice_type() {
        return true;
    }
    let Some(choices) = question.choices.as_ref() else {
        return false;
    };
    let Some(correct_choice_label) = question.correct_choice_label.as_ref() else {
        return false;
    };
    if choices.len() != 4 {
        return false;
    }
    let mut labels = HashSet::new();
    let mut texts = HashSet::new();
    let mut has_correct_choice = false;
    for choice in choices {
        if !has_display_content(&choice.label) || !has_display_content(&choice.text) {
            return false;
        }
        labels.insert(choice.label.as_str());
        texts.insert(choice.text.as_str());
        has_correct_choice |= choice.label == *correct_choice_label;
    }
    labels.len() == 4 && texts.len() == 4 && has_correct_choice
}

fn downgrade_choice_question_to_input(mut question: StudyQuestion) -> StudyQuestion {
    record_study_diagnostic(format!(
        "downgraded incomplete choice question to input: {} ({:?})",
        question.question_id, question.question_type
    ));
    let was_example_question = matches!(
        question.question_type,
        word_storage_core::models::QuestionType::ExampleToCnChoice
            | word_storage_core::models::QuestionType::ExampleToCnChoiceNoTranslation
    );
    question.question_type = word_storage_core::models::QuestionType::EnToCnInput;
    question.prompt = if was_example_question {
        question
            .example_sentence
            .clone()
            .unwrap_or_else(|| question.word.clone())
    } else {
        question.word.clone()
    };
    if !was_example_question {
        question.example_sentence = None;
        question.example_translation = None;
    }
    question.choices = None;
    question.correct_choice_label = None;
    question
}

fn valid_study_questions(questions: Vec<StudyQuestion>) -> Vec<StudyQuestion> {
    questions
        .into_iter()
        .filter(|question| {
            if valid_study_question(question) {
                true
            } else {
                record_study_diagnostic(format!(
                    "dropped invalid study question: {}",
                    question.question_id
                ));
                false
            }
        })
        .collect()
}

fn correct_choice_matches_question(question: &StudyQuestion, correct_choice_text: &str) -> bool {
    match question.question_type {
        word_storage_core::models::QuestionType::CnToEnChoice => {
            normalize_answer_text(correct_choice_text) == normalize_answer_text(&question.word)
        }
        _ => question.accepted_meanings.iter().any(|meaning| {
            let normalized_choice = normalize_answer_text(correct_choice_text);
            let normalized_meaning = normalize_answer_text(meaning);
            !normalized_choice.is_empty()
                && (normalized_choice == normalized_meaning
                    || normalized_meaning.contains(&normalized_choice)
                    || normalized_choice.contains(&normalized_meaning))
        }),
    }
}

fn normalize_answer_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn normalize_question_indexes(questions: &mut [StudyQuestion]) {
    let total_questions = questions.len() as u32;
    for (index, question) in questions.iter_mut().enumerate() {
        question.question_index = index as u32;
        question.total_questions = total_questions;
    }
}

/// Get the currently active study session state without mutating it.
pub fn get_active_study_session() -> Result<StartSessionResponse, StudyError> {
    let guard = lock_sessions();
    let active = guard.values().next().ok_or(StudyError::NoActiveSession)?;
    Ok(build_start_response(active))
}

pub fn get_study_session_answered_history(
    session_id: &str,
    before_question_index: usize,
    limit: usize,
) -> Result<Vec<AnsweredStudyQuestion>, StudyError> {
    let guard = lock_sessions();
    let active = guard
        .values()
        .find(|active| active.session.session_id == session_id)
        .ok_or(StudyError::NoActiveSession)?;
    Ok(answered_questions_before(
        active,
        before_question_index,
        limit,
    ))
}

/// Submit an answer for the current question.
pub fn accept_disputed_meaning(
    conn: &Connection,
    request: AcceptDisputedMeaningRequest,
) -> Result<AcceptDisputedMeaningResponse, StudyError> {
    let accepted_meaning = request.submitted_answer.trim().to_string();
    if accepted_meaning.is_empty() {
        return Err(StudyError::InvalidMode(
            "accepted meaning cannot be empty".to_string(),
        ));
    }

    {
        let mut guard = lock_sessions();
        if let Some(active) = guard
            .values_mut()
            .find(|session| session.question_map.contains_key(&request.question_id))
        {
            return accept_disputed_meaning_for_session(
                conn,
                active,
                request,
                accepted_meaning,
                false,
            );
        }
    }

    let mut completed_guard = lock_recent_completed_sessions();
    let completed = completed_guard
        .values_mut()
        .find(|session| session.question_map.contains_key(&request.question_id))
        .ok_or(StudyError::NoActiveSession)?;
    accept_disputed_meaning_for_session(conn, completed, request, accepted_meaning, true)
}

fn accept_disputed_meaning_for_session(
    conn: &Connection,
    active: &mut ActiveSession,
    request: AcceptDisputedMeaningRequest,
    accepted_meaning: String,
    session_completed: bool,
) -> Result<AcceptDisputedMeaningResponse, StudyError> {
    let question_index = active
        .question_map
        .get(&request.question_id)
        .copied()
        .ok_or(StudyError::NoActiveSession)?;
    let question = active.questions[question_index].clone();
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
        hint_used: false,
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
    let question_type = serde_json::to_string(&question.question_type)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    if let Err(error) = persistence::user_accepted_meaning_repo::save_user_dispute(
        conn,
        &question.entry_source_id,
        &result.question_id,
        &question_type,
        &accepted_meaning,
    ) {
        eprintln!("accepted meaning persistence failed but dispute result was accepted: {error}");
    }

    if session_completed {
        let completed_at = chrono::Utc::now().to_rfc3339();
        let summary = SessionSummaryService::build_summary_with_total(
            &active.session,
            &active.results,
            active.questions.len() as u32,
            &completed_at,
        );
        let next_action = SessionSummaryService::next_action(&summary, &active.session.mode);
        if let Err(error) = persistence::study_repo::save_completed_session(
            conn,
            &active.session,
            &summary,
            &active.results,
            &next_action,
        ) {
            eprintln!("study persistence failed after completed disputed meaning accept: {error}");
        }
    } else {
        persist_active_session(conn, active)?;
        if let Err(error) =
            persistence::study_repo::save_session_progress(conn, &active.session, &active.results)
        {
            eprintln!("study progress persistence failed after disputed meaning accept: {error}");
        }
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
    submit_study_answer_with_hint_usage(conn, request, false)
}

pub fn submit_study_answer_with_hint_usage(
    conn: &Connection,
    request: SubmitAnswerRequest,
    hint_used: bool,
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
    if let Some(existing_result) = active
        .results
        .iter()
        .find(|result| result.question_id == requested_question_id)
        .cloned()
    {
        return Ok(build_idempotent_submit_response(active, existing_result));
    }
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
    let mut result = AnswerEvaluator::evaluate(current_q, &answer, &answered_at);
    result.hint_used = hint_used;
    if active.session.mode == word_storage_core::models::SessionMode::HighFrequency
        && result.outcome.is_positive()
        && !result.hint_used
        && has_required_prior_high_frequency_correct_answers(conn, &result.entry_source_id)?
    {
        let mastery_reason = if high_frequency_entry_has_hint(conn, &result.entry_source_id)? {
            "high_frequency_five_correct_with_hint"
        } else {
            "high_frequency_three_correct"
        };
        if let Err(error) = persistence::mastered_entry_repo::mark_mastered_by_source_id(
            conn,
            &result.entry_source_id,
            mastery_reason,
        ) {
            eprintln!("failed to auto-master high-frequency entry: {error}");
        }
    }
    active.results.push(result.clone());

    // Advance
    active.current_index += 1;
    let is_complete = active.current_index >= active.questions.len();

    let (next_question, summary, next_action) = if is_complete {
        let completed_at = chrono::Utc::now().to_rfc3339();
        let summary = SessionSummaryService::build_summary_with_total(
            &active.session,
            &active.results,
            active.questions.len() as u32,
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
        lock_recent_completed_sessions().insert(mode_key.clone(), active.clone());
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

fn has_required_prior_high_frequency_correct_answers(
    conn: &Connection,
    entry_source_id: &str,
) -> Result<bool, StudyError> {
    let entry_id = conn
        .query_row(
            "SELECT id FROM entries WHERE source_entry_key = ?1 ORDER BY id ASC LIMIT 1",
            [entry_source_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| {
            StudyError::Storage(format!("Failed to resolve high-frequency entry: {error}"))
        })?;
    let required_prior_answers = if persistence::word_hint_repo::get_hint(conn, entry_id)
        .map_err(|error| StudyError::Storage(error.to_string()))?
        .is_some()
    {
        4
    } else {
        2
    };
    let mut statement = conn
        .prepare(
            "SELECT r.outcome
             FROM study_results r
             INNER JOIN study_sessions s ON s.session_id = r.session_id
             WHERE r.entry_id = ?1
               AND s.mode IN ('highFrequency', '\"highFrequency\"')
               AND COALESCE(r.hint_used, 0) = 0
             ORDER BY r.answered_at DESC, r.id DESC
             LIMIT ?2",
        )
        .map_err(|error| {
            StudyError::Storage(format!("Failed to prepare answer streak query: {error}"))
        })?;
    let outcomes = statement
        .query_map(rusqlite::params![entry_id, required_prior_answers], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| StudyError::Storage(format!("Failed to query answer streak: {error}")))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| StudyError::Storage(format!("Failed to read answer streak: {error}")))?;
    Ok(outcomes.len() == required_prior_answers as usize
        && outcomes.iter().all(|outcome| {
            matches!(
                outcome.trim_matches('"'),
                "correct" | "fuzzyCorrect" | "fuzzy_correct"
            )
        }))
}

fn high_frequency_entry_has_hint(
    conn: &Connection,
    entry_source_id: &str,
) -> Result<bool, StudyError> {
    let entry_id = conn
        .query_row(
            "SELECT id FROM entries WHERE source_entry_key = ?1 ORDER BY id ASC LIMIT 1",
            [entry_source_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| {
            StudyError::Storage(format!("Failed to resolve high-frequency entry: {error}"))
        })?;
    persistence::word_hint_repo::get_hint(conn, entry_id)
        .map(|hint| hint.is_some())
        .map_err(|error| StudyError::Storage(error.to_string()))
}

fn build_idempotent_submit_response(
    active: &ActiveSession,
    result: StudyResult,
) -> SubmitAnswerResponse {
    let is_complete = active.current_index >= active.questions.len();
    SubmitAnswerResponse {
        result,
        is_complete,
        current_question: None,
        summary: None,
        next_action: None,
        progress: SessionProgress {
            current: if is_complete {
                active.questions.len() as u32
            } else {
                active.current_index as u32 + 1
            },
            total: active.questions.len() as u32,
        },
        answered_questions: answered_questions(active),
    }
}

pub fn mark_study_entry_mastered(
    conn: &Connection,
    request: MarkStudyEntryMasteredRequest,
) -> Result<MarkStudyEntryMasteredResponse, StudyError> {
    mark_study_entry_mastered_with_replacements(conn, request, Vec::new())
}

pub fn mark_study_entry_mastered_with_replacements(
    conn: &Connection,
    request: MarkStudyEntryMasteredRequest,
    replacement_entry_payloads: Vec<StartSessionEntryPayload>,
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
            active
                .questions
                .iter()
                .any(|question| question.entry_source_id == request.entry_source_id)
        })
        .map(|(mode, _)| mode.clone());
    let Some(mode_key) = mode_key else {
        drop(guard);
        let completed_guard = lock_recent_completed_sessions();
        let active = completed_guard
            .values()
            .find(|active| {
                active
                    .questions
                    .iter()
                    .any(|question| question.entry_source_id == request.entry_source_id)
            })
            .ok_or(StudyError::NoActiveSession)?;
        let completed_at = chrono::Utc::now().to_rfc3339();
        let summary = SessionSummaryService::build_summary_with_total(
            &active.session,
            &active.results,
            active.questions.len() as u32,
            &completed_at,
        );
        let next_action = SessionSummaryService::next_action(&summary, &active.session.mode);
        return Ok(MarkStudyEntryMasteredResponse {
            entry_source_id: request.entry_source_id,
            entry_id,
            pruned_question_count: 0,
            is_complete: true,
            current_question: None,
            summary: Some(summary),
            next_action: Some(next_action),
            progress: SessionProgress {
                current: active.questions.len() as u32,
                total: active.questions.len() as u32,
            },
            answered_questions: answered_questions(active),
        });
    };
    let active = guard
        .get_mut(&mode_key)
        .ok_or(StudyError::NoActiveSession)?;
    let answered_ids = active
        .results
        .iter()
        .map(|result| result.question_id.clone())
        .collect::<std::collections::HashSet<_>>();
    let entry_was_answered = active.questions.iter().any(|question| {
        question.entry_source_id == request.entry_source_id
            && answered_ids.contains(&question.question_id)
    });
    let before = active.questions.len();
    active.questions.retain(|question| {
        question.entry_source_id != request.entry_source_id
            || answered_ids.contains(&question.question_id)
    });
    let pruned_question_count = before.saturating_sub(active.questions.len()) as u32;
    if pruned_question_count > 0 {
        active.question_plan_mutated = true;
    }
    if !entry_was_answered && pruned_question_count > 0 {
        active
            .entry_source_ids
            .retain(|source_id| source_id != &request.entry_source_id);
        active
            .entry_payloads
            .retain(|payload| payload.source_id != request.entry_source_id);

        let existing_sources = active
            .entry_source_ids
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        let replacements = valid_entry_payloads(&replacement_entry_payloads)
            .into_iter()
            .filter(|payload| {
                payload.source_id != request.entry_source_id
                    && !existing_sources.contains(&payload.source_id)
            })
            .collect::<Vec<_>>();
        if !replacements.is_empty() {
            let replacement_words = payloads_to_words(&replacements);
            let mut distractor_payloads = active.entry_payloads.clone();
            distractor_payloads.extend(active.distractor_payloads.clone());
            distractor_payloads.extend(replacements.clone());
            let distractors = payloads_to_words(&distractor_payloads);
            let replacement_session_id = format!(
                "{}_replacement_{}_{}",
                active.session.session_id,
                active.results.len(),
                replacements[0].source_id
            );
            let mut replacement_questions = QuestionBuilder::build_session_questions(
                &active.session.mode,
                &replacement_words,
                &distractors,
                &replacement_session_id,
                &active.question_type_weights,
            );
            replacement_questions = repair_study_questions(replacement_questions);
            replacement_questions = valid_study_questions(replacement_questions);
            active.questions.extend(replacement_questions);
            for replacement in replacements {
                active.entry_source_ids.push(replacement.source_id.clone());
                active.entry_payloads.push(replacement);
            }
        }
        if !active.entry_payloads.is_empty() {
            active.session.total_words = active.entry_payloads.len() as u32;
        } else {
            active.session.total_words = active.entry_source_ids.len() as u32;
        }
    }
    normalize_question_indexes(&mut active.questions);
    rebuild_question_map(active);
    active.current_index = active
        .questions
        .iter()
        .position(|question| !answered_ids.contains(&question.question_id))
        .unwrap_or(active.questions.len());

    let is_complete = active.current_index >= active.questions.len();
    let (current_question, summary, next_action) = if is_complete {
        let completed_at = chrono::Utc::now().to_rfc3339();
        let summary = SessionSummaryService::build_summary_with_total(
            &active.session,
            &active.results,
            active.questions.len() as u32,
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
        lock_recent_completed_sessions().insert(mode_key.clone(), active.clone());
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
    let summary = SessionSummaryService::build_summary_with_total(
        &active.session,
        &active.results,
        active.questions.len() as u32,
        &completed_at,
    );
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
    let start = active.results.len().saturating_sub(ANSWERED_FEED_WINDOW);
    answered_questions_range(active, start, active.results.len())
}

fn answered_questions_before(
    active: &ActiveSession,
    before_question_index: usize,
    limit: usize,
) -> Vec<AnsweredStudyQuestion> {
    let mut items = active
        .results
        .iter()
        .filter_map(|result| {
            let question_index = active.question_map.get(&result.question_id).copied()?;
            if question_index >= before_question_index {
                return None;
            }
            let question = active.questions.get(question_index)?.clone();
            Some(AnsweredStudyQuestion {
                question,
                result: result.clone(),
            })
        })
        .collect::<Vec<_>>();
    if items.len() > limit {
        items.drain(0..items.len() - limit);
    }
    items
}

fn answered_questions_range(
    active: &ActiveSession,
    start: usize,
    end: usize,
) -> Vec<AnsweredStudyQuestion> {
    active
        .results
        .iter()
        .skip(start)
        .take(end.saturating_sub(start))
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

fn snapshot_rejection_reason(
    snapshot: &ActiveSessionSnapshotV1,
    mode: &word_storage_core::models::SessionMode,
    expected_questions: usize,
) -> Option<SnapshotRejectionReason> {
    if snapshot.question_engine_version != QUESTION_ENGINE_VERSION {
        return Some(SnapshotRejectionReason::EngineVersion);
    }
    let shape_stale = snapshot.questions.len() != expected_questions
        || snapshot.current_index >= snapshot.questions.len()
        || snapshot_contains_restore_placeholders(snapshot)
        || snapshot_contains_invalid_questions(snapshot)
        || persisted_session_snapshot_stale(snapshot, mode);
    if shape_stale {
        return Some(SnapshotRejectionReason::Shape);
    }
    None
}

fn v2_snapshot_matches_request(
    snapshot: &ActiveSessionSnapshotV2,
    request: &StartSessionRequest,
    question_type_weights: &[QuestionTypeWeight],
) -> bool {
    if request.entry_source_ids.is_empty() && request.entry_payloads.is_empty() {
        return true;
    }
    if snapshot.question_type_weights != question_type_weights {
        return false;
    }
    let requested = request_source_id_set(request);
    snapshot
        .entry_source_ids
        .iter()
        .all(|source_id| requested.contains(source_id))
}

fn snapshot_question_weights_stale(
    snapshot: &ActiveSessionSnapshotV1,
    question_type_weights: &[QuestionTypeWeight],
) -> bool {
    snapshot.question_type_weights != question_type_weights
}

fn snapshot_contains_restore_placeholders(snapshot: &ActiveSessionSnapshotV1) -> bool {
    snapshot.questions.iter().any(|question| {
        question
            .entry_source_id
            .starts_with("active_session_restore_")
            || question.word.starts_with("active_session_restore_")
            || question.prompt.starts_with("active_session_restore_")
    })
}

fn snapshot_contains_invalid_questions(snapshot: &ActiveSessionSnapshotV1) -> bool {
    snapshot.questions.iter().any(|question| {
        !has_study_word_content(&question.word)
            || !has_display_content(&question.prompt)
            || !has_display_content(&question.entry_source_id)
            || question
                .accepted_meanings
                .iter()
                .all(|meaning| !has_display_content(meaning))
            || question.choices.as_ref().is_some_and(|choices| {
                choices.iter().any(|choice| {
                    !has_display_content(&choice.label) || !has_display_content(&choice.text)
                })
            })
    })
}

fn has_display_content(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return false;
    }
    trimmed.chars().any(|ch| ch.is_alphanumeric())
}

fn has_study_word_content(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains('.') || trimmed == "/" {
        return false;
    }
    trimmed.chars().filter(|ch| ch.is_alphabetic()).count() >= 2
}

fn persisted_session_snapshot_stale(
    snapshot: &ActiveSessionSnapshotV1,
    mode: &word_storage_core::models::SessionMode,
) -> bool {
    snapshot.session.mode != *mode || !session_started_today(&snapshot.session)
}

fn active_session_snapshot_stale(
    active: &ActiveSession,
    mode: &word_storage_core::models::SessionMode,
) -> bool {
    active.session.mode != *mode || !session_started_today(&active.session)
}

fn session_started_today(session: &StudySession) -> bool {
    let Ok(started_at) = chrono::DateTime::parse_from_rfc3339(&session.started_at) else {
        return false;
    };
    started_at.with_timezone(&chrono::Local).date_naive() == chrono::Local::now().date_naive()
}

fn snapshot_matches_request(
    snapshot: &ActiveSessionSnapshotV1,
    request: &StartSessionRequest,
    question_type_weights: &[QuestionTypeWeight],
) -> bool {
    if request.entry_source_ids.is_empty() && request.entry_payloads.is_empty() {
        return true;
    }
    if snapshot.question_type_weights != question_type_weights {
        return false;
    }
    let requested = request_source_id_set(request);
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
    let requested = request_source_id_set(request);
    active
        .questions
        .iter()
        .all(|question| requested.contains(&question.entry_source_id))
}

fn request_source_id_set(request: &StartSessionRequest) -> HashSet<&String> {
    if request.entry_payloads.is_empty() {
        request.entry_source_ids.iter().collect::<HashSet<_>>()
    } else {
        request
            .entry_payloads
            .iter()
            .map(|payload| &payload.source_id)
            .collect::<HashSet<_>>()
    }
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

fn active_session_from_snapshot(snapshot: ActiveSessionSnapshotV1) -> ActiveSession {
    let mut question_map = HashMap::new();
    for (i, question) in snapshot.questions.iter().enumerate() {
        question_map.insert(question.question_id.clone(), i);
    }
    let mut entry_source_ids = Vec::new();
    for question in &snapshot.questions {
        if !entry_source_ids.contains(&question.entry_source_id) {
            entry_source_ids.push(question.entry_source_id.clone());
        }
    }
    ActiveSession {
        session: snapshot.session,
        questions: snapshot.questions,
        results: snapshot.results,
        current_index: snapshot.current_index,
        question_map,
        question_type_weights: snapshot.question_type_weights,
        entry_source_ids,
        entry_payloads: Vec::new(),
        distractor_payloads: Vec::new(),
        question_plan_mutated: false,
    }
}

fn active_session_from_v2_snapshot(
    snapshot: ActiveSessionSnapshotV2,
    mode: &word_storage_core::models::SessionMode,
) -> Option<ActiveSession> {
    if snapshot.session.mode != *mode
        || !session_started_today(&snapshot.session)
        || snapshot.entry_payloads.is_empty()
        || snapshot.question_engine_version != QUESTION_ENGINE_VERSION
        || snapshot.schema_version != ACTIVE_SNAPSHOT_SCHEMA_VERSION_V2
    {
        return None;
    }

    let entry_payloads = valid_entry_payloads(&snapshot.entry_payloads);
    if entry_payloads.is_empty() {
        return None;
    }
    let distractor_payloads = valid_entry_payloads(&snapshot.distractor_payloads);
    let question_plan_mutated = !snapshot.questions.is_empty();
    let mut questions = if !question_plan_mutated {
        let words = payloads_to_words(&entry_payloads);
        let distractors = payloads_to_words(&distractor_payloads);
        QuestionBuilder::build_session_questions(
            &snapshot.session.mode,
            &words,
            &distractors,
            &snapshot.session.session_id,
            &snapshot.question_type_weights,
        )
    } else {
        snapshot.questions
    };
    questions = repair_study_questions(questions);
    questions = valid_study_questions(questions);
    normalize_question_indexes(&mut questions);
    if questions.is_empty() || snapshot.current_index >= questions.len() {
        return None;
    }

    let mut question_map = HashMap::new();
    for (i, question) in questions.iter().enumerate() {
        question_map.insert(question.question_id.clone(), i);
    }

    Some(ActiveSession {
        session: snapshot.session,
        questions,
        results: snapshot.results,
        current_index: snapshot.current_index,
        question_map,
        question_type_weights: snapshot.question_type_weights,
        entry_source_ids: snapshot.entry_source_ids,
        entry_payloads,
        distractor_payloads,
        question_plan_mutated,
    })
}

fn load_persisted_session(
    conn: &Connection,
    mode: &word_storage_core::models::SessionMode,
) -> Result<Option<LoadedActiveSessionSnapshot>, StudyError> {
    let Some(raw) = persistence::study_repo::load_active_session_snapshot(conn, mode)
        .map_err(|e| StudyError::Storage(e.to_string()))?
    else {
        return Ok(None);
    };
    match decode_active_session_snapshot(&raw) {
        Ok(snapshot) => Ok(Some(snapshot)),
        Err(error) => {
            record_study_diagnostic(format!(
                "cleared undecodable active snapshot for mode {}: {error}",
                mode_storage_key(mode)
            ));
            clear_persisted_session(conn, mode)?;
            Ok(None)
        }
    }
}

fn decode_active_session_snapshot(raw: &str) -> serde_json::Result<LoadedActiveSessionSnapshot> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(|version| version.as_i64())
        .unwrap_or(ACTIVE_SNAPSHOT_SCHEMA_VERSION_V1);
    if schema_version == ACTIVE_SNAPSHOT_SCHEMA_VERSION_V2 {
        serde_json::from_value(value).map(LoadedActiveSessionSnapshot::V2)
    } else {
        serde_json::from_value(value).map(LoadedActiveSessionSnapshot::V1)
    }
}

fn persist_active_session(conn: &Connection, active: &ActiveSession) -> Result<(), StudyError> {
    let payload = serde_json::to_string(&active_session_snapshot_payload(active))
        .map_err(|e| StudyError::Storage(format!("Failed to serialize active session: {e}")))?;
    persistence::study_repo::save_active_session_snapshot(conn, &active.session.mode, &payload)
        .map_err(|e| StudyError::Storage(e.to_string()))
}

fn active_session_snapshot_payload(active: &ActiveSession) -> serde_json::Value {
    if !active.entry_payloads.is_empty() {
        serde_json::to_value(ActiveSessionSnapshotV2 {
            schema_version: ACTIVE_SNAPSHOT_SCHEMA_VERSION_V2,
            question_engine_version: QUESTION_ENGINE_VERSION,
            vocabulary_version: None,
            session: active.session.clone(),
            study_date: session_study_date(&active.session),
            entry_source_ids: active.entry_source_ids.clone(),
            entry_payloads: active.entry_payloads.clone(),
            distractor_payloads: active.distractor_payloads.clone(),
            question_plan_signature: question_plan_signature(active),
            questions: if active.question_plan_mutated {
                active.questions.clone()
            } else {
                Vec::new()
            },
            current_index: active.current_index,
            results: active.results.clone(),
            question_type_weights: active.question_type_weights.clone(),
        })
        .unwrap_or(serde_json::Value::Null)
    } else {
        serde_json::to_value(ActiveSessionSnapshotV1 {
            schema_version: ACTIVE_SNAPSHOT_SCHEMA_VERSION_V1,
            question_engine_version: QUESTION_ENGINE_VERSION,
            session: active.session.clone(),
            questions: active.questions.clone(),
            results: active.results.clone(),
            current_index: active.current_index,
            question_type_weights: active.question_type_weights.clone(),
        })
        .unwrap_or(serde_json::Value::Null)
    }
}

fn session_study_date(session: &StudySession) -> String {
    chrono::DateTime::parse_from_rfc3339(&session.started_at)
        .map(|dt| dt.with_timezone(&chrono::Local).date_naive().to_string())
        .unwrap_or_else(|_| chrono::Local::now().date_naive().to_string())
}

fn question_plan_signature(active: &ActiveSession) -> String {
    let ids = if active.entry_source_ids.is_empty() {
        active
            .entry_payloads
            .iter()
            .map(|payload| payload.source_id.as_str())
            .collect::<Vec<_>>()
            .join(",")
    } else {
        active.entry_source_ids.join(",")
    };
    let weights = active
        .question_type_weights
        .iter()
        .map(|weight| format!("{:?}:{}", weight.question_type, weight.weight))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}|{}|{}",
        mode_storage_key(&active.session.mode),
        ids,
        weights
    )
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
        accept_disputed_meaning, clear_all_active_sessions,
        has_required_prior_high_frequency_correct_answers, mark_study_entry_mastered,
        mark_study_entry_mastered_with_replacements, start_study_session, submit_study_answer,
        StudyError,
    };
    use rusqlite::OptionalExtension;
    use word_storage_core::models::{
        AcceptDisputedMeaningRequest, AnswerOutcome, ChoiceOption, MarkStudyEntryMasteredRequest,
        QuestionType, QuestionTypeWeight, SessionMode, StartSessionEntryPayload,
        StartSessionMeaningPayload, StartSessionRequest, StudyQuestion, SubmitAnswerRequest,
    };
    use word_storage_core::persistence::study_repo;

    #[test]
    fn high_frequency_streak_requires_more_correct_answers_when_a_hint_exists() {
        let conn = rusqlite::Connection::open_in_memory().expect("open database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status) VALUES (1, 'streak-test', 'ready')",
            [],
        )
        .expect("source");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma)
             VALUES (1, 1, 'streak-word', 'state', 'state')",
            [],
        )
        .expect("entry");
        for (index, outcome) in ["\"correct\"", "\"fuzzyCorrect\""].into_iter().enumerate() {
            let session_id = format!("streak-session-{index}");
            conn.execute(
                "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
                 VALUES (?1, '\"highFrequency\"', 1, datetime('now'), datetime('now'))",
                [&session_id],
            )
            .expect("session");
            conn.execute(
                "INSERT INTO study_results (session_id, question_id, entry_id, question_type,
                 user_response, correct_answer, outcome, response_time_ms, answered_at)
                 VALUES (?1, ?2, 1, '\"enToCnInput\"', 'x', 'x', ?3, 1, ?4)",
                rusqlite::params![
                    session_id,
                    format!("q-{index}"),
                    outcome,
                    format!("2026-08-06T00:00:0{index}Z")
                ],
            )
            .expect("result");
        }
        assert!(
            has_required_prior_high_frequency_correct_answers(&conn, "streak-word")
                .expect("positive streak")
        );

        word_storage_core::persistence::word_hint_repo::save_hint(
            &conn,
            1,
            "remember this",
            "user",
        )
        .expect("save hint");
        assert!(
            !has_required_prior_high_frequency_correct_answers(&conn, "streak-word")
                .expect("hinted streak needs four prior answers")
        );
        for index in 2..4 {
            let session_id = format!("streak-session-{index}");
            conn.execute(
                "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
                 VALUES (?1, '\"highFrequency\"', 1, datetime('now'), datetime('now'))",
                [&session_id],
            )
            .expect("session");
            conn.execute(
                "INSERT INTO study_results (session_id, question_id, entry_id, question_type,
                 user_response, correct_answer, outcome, response_time_ms, answered_at)
                 VALUES (?1, ?2, 1, '\"enToCnInput\"', 'x', 'x', '\"correct\"', 1, ?3)",
                rusqlite::params![
                    session_id,
                    format!("q-{index}"),
                    format!("2026-08-06T00:00:0{index}Z")
                ],
            )
            .expect("result");
        }
        assert!(
            has_required_prior_high_frequency_correct_answers(&conn, "streak-word")
                .expect("four prior answers satisfy hinted streak")
        );

        conn.execute(
            "UPDATE study_results SET hint_used = 1 WHERE question_id = 'q-3'",
            [],
        )
        .expect("mark one correct answer as hinted");
        assert!(
            !has_required_prior_high_frequency_correct_answers(&conn, "streak-word")
                .expect("hinted correct answer adds no mastery progress")
        );
        conn.execute(
            "UPDATE study_results SET hint_used = 0 WHERE question_id = 'q-3'",
            [],
        )
        .expect("restore unhinted streak");

        conn.execute(
            "UPDATE study_results SET outcome = '\"incorrect\"' WHERE question_id = 'q-1'",
            [],
        )
        .expect("break streak");
        assert!(
            !has_required_prior_high_frequency_correct_answers(&conn, "streak-word")
                .expect("broken streak")
        );
    }

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
            QuestionType::EnToCnInput
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
    fn start_session_discards_v2_snapshot_without_payloads() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();
        let started_at = chrono::Local::now().to_rfc3339();
        let study_date = chrono::Local::now().date_naive().to_string();

        let snapshot = serde_json::json!({
            "schemaVersion": super::ACTIVE_SNAPSHOT_SCHEMA_VERSION_V2,
            "questionEngineVersion": super::QUESTION_ENGINE_VERSION,
            "vocabularyVersion": "seed-vocab-test",
            "session": {
                "sessionId": "sess_v2_resume",
                "mode": "review",
                "totalWords": 28,
                "wordbookId": null,
                "startedAt": started_at
            },
            "studyDate": study_date,
            "entrySourceIds": ["review_entry_0", "review_entry_1"],
            "questionPlanSignature": "review:test-plan",
            "currentIndex": 16,
            "results": [],
            "questionTypeWeights": [{
                "questionType": "cnToEnChoice",
                "weight": 100
            }]
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_review", snapshot.to_string()),
        )
        .expect("insert v2 active snapshot");

        let error = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::Review,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect_err("v2 snapshot without payloads cannot rebuild");

        assert!(matches!(error, StudyError::NotEnoughWords));
        let remaining_snapshot =
            study_repo::load_active_session_snapshot(&conn, &SessionMode::Review)
                .expect("load snapshot after v2 rejection");
        assert!(remaining_snapshot.is_none());
    }

    #[test]
    fn start_session_rebuilds_after_invalid_v2_snapshot_when_request_has_payloads() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let snapshot = serde_json::json!({
            "schemaVersion": super::ACTIVE_SNAPSHOT_SCHEMA_VERSION_V2,
            "questionEngineVersion": super::QUESTION_ENGINE_VERSION,
            "session": {
                "sessionId": "sess_v2_corrupt",
                "mode": "review",
                "totalWords": 1,
                "wordbookId": null,
                "startedAt": chrono::Local::now().to_rfc3339()
            },
            "studyDate": chrono::Local::now().date_naive().to_string(),
            "entrySourceIds": ["stale_payload"],
            "entryPayloads": [],
            "distractorPayloads": [],
            "questionPlanSignature": "review|stale_payload|",
            "currentIndex": 0,
            "results": [],
            "questionTypeWeights": []
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_review", snapshot.to_string()),
        )
        .expect("insert corrupt v2 snapshot");

        let response = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::Review,
                wordbook_id: None,
                entry_source_ids: vec!["fresh_entry".to_string()],
                entry_payloads: vec![test_entry("fresh_entry", "debate", "discussion")],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("fresh request should rebuild after corrupt v2 snapshot");

        assert_eq!(response.current_question.entry_source_id, "fresh_entry");
        assert_ne!(response.session.session_id, "sess_v2_corrupt");
        assert!(super::recent_study_diagnostics()
            .iter()
            .any(|item| item.contains("cleared") && item.contains("v2 active snapshot")));
    }

    #[test]
    fn start_session_clears_undecodable_snapshot_and_uses_fresh_payloads() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_mixedTest", "{not-valid-json"),
        )
        .expect("insert undecodable snapshot");

        let response = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["fresh_mixed".to_string()],
                entry_payloads: vec![test_entry("fresh_mixed", "alpha", "first")],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnInput,
                    weight: 100,
                }],
            },
        )
        .expect("fresh request should recover from undecodable snapshot");

        assert_eq!(response.current_question.entry_source_id, "fresh_mixed");
        let remaining_snapshot =
            study_repo::load_active_session_snapshot(&conn, &SessionMode::MixedTest)
                .expect("load replacement snapshot");
        assert!(remaining_snapshot.is_some());
        assert!(super::recent_study_diagnostics()
            .iter()
            .any(|item| item.contains("cleared undecodable active snapshot")));
    }

    #[test]
    fn start_session_persists_and_restores_v2_snapshot_from_payloads() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let payloads = vec![
            test_entry("payload_one", "debate", "杈╄"),
            test_entry("payload_two", "opinion", "瑙傜偣"),
        ];

        let first = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: payloads
                    .iter()
                    .map(|payload| payload.source_id.clone())
                    .collect(),
                entry_payloads: payloads,
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnInput,
                    weight: 100,
                }],
            },
        )
        .expect("start v2-backed session");

        let raw_snapshot = study_repo::load_active_session_snapshot(&conn, &SessionMode::MixedTest)
            .expect("load persisted snapshot")
            .expect("snapshot should exist");
        assert!(raw_snapshot.contains("\"schemaVersion\":2"));
        assert!(!raw_snapshot.contains("\"questions\""));

        clear_all_active_sessions();

        let restored = start_study_session(
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
        .expect("restore v2-backed session");

        assert_eq!(restored.session.session_id, first.session.session_id);
        assert_eq!(
            restored.current_question.question_id,
            first.current_question.question_id
        );
        assert_eq!(restored.current_question.word, first.current_question.word);
    }

    #[test]
    fn start_session_restores_v2_snapshot_progress_after_answer() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let payloads = vec![
            test_entry("payload_one", "debate", "杈╄"),
            test_entry("payload_two", "opinion", "瑙傜偣"),
        ];

        let first = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: payloads
                    .iter()
                    .map(|payload| payload.source_id.clone())
                    .collect(),
                entry_payloads: payloads,
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnInput,
                    weight: 100,
                }],
            },
        )
        .expect("start v2-backed session");
        submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: first.current_question.question_id.clone(),
                response: first.current_question.accepted_meanings[0].clone(),
                response_time_ms: 1200,
            },
        )
        .expect("submit first answer");

        clear_all_active_sessions();

        let restored = start_study_session(
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
        .expect("restore v2-backed progress");

        assert_eq!(restored.progress.current, 2);
        assert_eq!(restored.answered_questions.len(), 1);
        assert_ne!(
            restored.current_question.question_id,
            first.current_question.question_id
        );
    }

    #[test]
    fn start_session_filters_invalid_entry_payloads_before_generation() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let mut invalid = test_entry("invalid_entry", "/", "/");
        invalid.meaning_details.clear();
        invalid.meanings = vec!["/".to_string()];
        let valid = test_entry("valid_entry", "debate", "杈╄");

        let response = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["invalid_entry".to_string(), "valid_entry".to_string()],
                entry_payloads: vec![invalid, valid],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnInput,
                    weight: 100,
                }],
            },
        )
        .expect("valid payload should still start after invalid entry is skipped");

        assert_eq!(response.session.total_words, 1);
        assert_eq!(response.current_question.entry_source_id, "valid_entry");
        assert_eq!(response.current_question.word, "debate");
        assert_ne!(response.current_question.prompt, "/");
    }

    #[test]
    fn start_session_filters_abbreviation_like_entry_payloads_before_generation() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let dotted = test_entry("bad_abbreviation", "a.", "catholic");
        let short = test_entry("bad_single_letter", "a", "letter");
        let valid = test_entry("valid_entry", "debate", "discussion");

        let response = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec![
                    "bad_abbreviation".to_string(),
                    "bad_single_letter".to_string(),
                    "valid_entry".to_string(),
                ],
                entry_payloads: vec![dotted, short, valid],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::ExampleToCnChoice,
                    weight: 100,
                }],
            },
        )
        .expect("valid payload should still start after abbreviation-like entries are skipped");

        assert_eq!(response.session.total_words, 1);
        assert_eq!(response.current_question.entry_source_id, "valid_entry");
        assert_eq!(response.current_question.word, "debate");
        assert_ne!(response.current_question.prompt, "a.");
    }

    #[test]
    fn start_session_filters_empty_choice_distractors_before_validation() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let mut payload = test_entry("choice_entry", "gesture", "鎵嬪娍");
        payload.cn_choice_distractors = vec![
            "/".to_string(),
            " ".to_string(),
            "寤鸿".to_string(),
            "闇€姹�".to_string(),
            "搴撳瓨".to_string(),
        ];

        let response = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["choice_entry".to_string()],
                entry_payloads: vec![payload],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnChoice,
                    weight: 100,
                }],
            },
        )
        .expect("choice session should start after empty distractors are filtered");

        let choices = response
            .current_question
            .choices
            .expect("choice question should include options");
        assert!(choices.iter().all(|choice| choice.text.trim() != "/"));
        assert!(choices.iter().all(|choice| !choice.text.trim().is_empty()));
    }

    #[test]
    fn start_session_downgrades_incomplete_choice_question_without_shrinking_session() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let mut payload = test_entry("choice_entry", "adjacent", "nearby");
        payload.cn_choice_distractors = vec!["raw".to_string(), "hard".to_string()];

        let response = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["choice_entry".to_string()],
                entry_payloads: vec![payload],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnChoice,
                    weight: 100,
                }],
            },
        )
        .expect("incomplete choice should be downgraded instead of dropped");

        assert_eq!(
            response.current_question.question_type,
            QuestionType::EnToCnInput
        );
        assert!(response.current_question.choices.is_none());
        assert_eq!(response.current_question.prompt, "adjacent");
        assert_eq!(response.current_question.total_questions, 1);
        assert_eq!(response.progress.total, 1);
    }

    #[test]
    fn incomplete_example_choice_keeps_its_sentence_when_downgraded() {
        let question = StudyQuestion {
            question_id: "example_choice".to_string(),
            question_type: QuestionType::ExampleToCnChoice,
            entry_source_id: "survive".to_string(),
            word: "survive".to_string(),
            part_of_speech: Some("v".to_string()),
            phonetic_us: None,
            phonetic_uk: None,
            prompt: "Only the strongest plants survive.".to_string(),
            accepted_meanings: vec!["生存".to_string()],
            example_sentence: Some("Only the strongest plants survive.".to_string()),
            example_translation: None,
            choices: Some(Vec::new()),
            correct_choice_label: None,
            question_index: 0,
            total_questions: 1,
        };

        let repaired = super::repair_study_questions(vec![question]);

        assert_eq!(repaired[0].question_type, QuestionType::EnToCnInput);
        assert_eq!(repaired[0].prompt, "Only the strongest plants survive.");
        assert_eq!(
            repaired[0].example_sentence.as_deref(),
            Some("Only the strongest plants survive.")
        );
    }

    #[test]
    fn study_question_validation_rejects_mismatched_correct_choice_label() {
        let question = StudyQuestion {
            question_id: "q_choice_conflict".to_string(),
            question_type: QuestionType::EnToCnChoice,
            entry_source_id: "condemn_entry".to_string(),
            word: "condemn".to_string(),
            part_of_speech: Some("vt".to_string()),
            phonetic_us: None,
            phonetic_uk: None,
            prompt: "condemn".to_string(),
            accepted_meanings: vec!["璋磋矗锛屾寚璐ｏ紱鍒ゅ垜锛屽鍛婃湁缃�".to_string()],
            example_sentence: None,
            example_translation: None,
            choices: Some(vec![
                ChoiceOption {
                    label: "A".to_string(),
                    text: "鎹熷潖锛岀牬鍧忥紱瀹犲潖锛屾汉鐖�".to_string(),
                },
                ChoiceOption {
                    label: "B".to_string(),
                    text: "璋磋矗锛屾寚璐ｏ紱鍒ゅ垜锛屽鍛婃湁缃�".to_string(),
                },
            ]),
            correct_choice_label: Some("A".to_string()),
            question_index: 0,
            total_questions: 1,
        };

        assert!(!super::valid_study_question(&question));
    }

    #[test]
    fn study_question_validation_rejects_abbreviation_like_word() {
        let question = StudyQuestion {
            question_id: "q_abbrev".to_string(),
            question_type: QuestionType::ExampleToCnChoice,
            entry_source_id: "kaoyan_bad_abbrev".to_string(),
            word: "a.".to_string(),
            part_of_speech: Some("n".to_string()),
            phonetic_us: None,
            phonetic_uk: None,
            prompt: "a.".to_string(),
            accepted_meanings: vec!["catholic".to_string()],
            example_sentence: Some("a.".to_string()),
            example_translation: None,
            choices: Some(vec![ChoiceOption {
                label: "A".to_string(),
                text: "catholic".to_string(),
            }]),
            correct_choice_label: Some("A".to_string()),
            question_index: 0,
            total_questions: 1,
        };

        assert!(!super::valid_study_question(&question));
        assert!(super::has_study_word_content("resumé"));
    }

    #[test]
    fn start_session_discards_previous_day_snapshot_for_empty_resume_request() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();
        let started_at = (chrono::Local::now() - chrono::Duration::days(1)).to_rfc3339();
        let questions = (0..28)
            .map(|index| {
                serde_json::to_value(StudyQuestion {
                    question_id: format!("sess_old_review_{index}"),
                    question_type: QuestionType::CnToEnChoice,
                    entry_source_id: format!("review_entry_{index}"),
                    word: format!("review_word_{index}"),
                    part_of_speech: None,
                    phonetic_us: None,
                    phonetic_uk: None,
                    prompt: format!("review meaning {index}"),
                    accepted_meanings: vec![format!("review meaning {index}")],
                    example_sentence: None,
                    example_translation: None,
                    choices: Some(vec![ChoiceOption {
                        text: format!("review_word_{index}"),
                        label: "A".to_string(),
                    }]),
                    correct_choice_label: Some("A".to_string()),
                    question_index: index as u32,
                    total_questions: 28,
                })
                .expect("serialize question")
            })
            .collect::<Vec<_>>();

        let snapshot = serde_json::json!({
            "questionEngineVersion": super::QUESTION_ENGINE_VERSION,
            "session": {
                "sessionId": "sess_old_review",
                "mode": "review",
                "totalWords": 28,
                "wordbookId": null,
                "startedAt": started_at
            },
            "questions": questions,
            "results": [],
            "currentIndex": 15,
            "questionTypeWeights": [{
                "questionType": "cnToEnChoice",
                "weight": 100
            }]
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_review", snapshot.to_string()),
        )
        .expect("insert previous-day active snapshot");

        let error = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::Review,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect_err("empty start request must not restore previous-day progress");

        assert!(matches!(error, StudyError::NotEnoughWords));
        let remaining_snapshot =
            study_repo::load_active_session_snapshot(&conn, &SessionMode::Review)
                .expect("load snapshot");
        assert!(remaining_snapshot.is_none());
    }

    #[test]
    fn start_session_discards_snapshot_with_invalid_question_prompt() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();
        let started_at = chrono::Local::now().to_rfc3339();
        let snapshot = serde_json::json!({
            "questionEngineVersion": super::QUESTION_ENGINE_VERSION,
            "session": {
                "sessionId": "sess_bad_prompt",
                "mode": "review",
                "totalWords": 1,
                "wordbookId": null,
                "startedAt": started_at
            },
            "questions": [serde_json::to_value(StudyQuestion {
                question_id: "sess_bad_prompt_0".to_string(),
                question_type: QuestionType::CnToEnChoice,
                entry_source_id: "debate".to_string(),
                word: "/".to_string(),
                part_of_speech: None,
                phonetic_us: Some("dɪˈbet".to_string()),
                phonetic_uk: None,
                prompt: "/".to_string(),
                accepted_meanings: vec!["debate".to_string()],
                example_sentence: None,
                example_translation: None,
                choices: Some(vec![ChoiceOption {
                    text: "debate".to_string(),
                    label: "A".to_string(),
                }]),
                correct_choice_label: Some("A".to_string()),
                question_index: 15,
                total_questions: 28,
            }).expect("serialize question")],
            "results": [],
            "currentIndex": 0,
            "questionTypeWeights": [{
                "questionType": "cnToEnChoice",
                "weight": 100
            }]
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_review", snapshot.to_string()),
        )
        .expect("insert invalid active snapshot");

        let error = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::Review,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect_err("empty start request must not restore invalid question prompt");

        assert!(matches!(error, StudyError::NotEnoughWords));
        let remaining_snapshot =
            study_repo::load_active_session_snapshot(&conn, &SessionMode::Review)
                .expect("load snapshot");
        assert!(remaining_snapshot.is_none());
    }

    #[test]
    fn start_session_discards_snapshot_with_empty_choice_text() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();
        let started_at = chrono::Local::now().to_rfc3339();
        let snapshot = serde_json::json!({
            "questionEngineVersion": super::QUESTION_ENGINE_VERSION,
            "session": {
                "sessionId": "sess_empty_choice",
                "mode": "review",
                "totalWords": 1,
                "wordbookId": null,
                "startedAt": started_at
            },
            "questions": [serde_json::to_value(StudyQuestion {
                question_id: "sess_empty_choice_0".to_string(),
                question_type: QuestionType::CnToEnChoice,
                entry_source_id: "debate".to_string(),
                word: "debate".to_string(),
                part_of_speech: None,
                phonetic_us: Some("dɪˈbet".to_string()),
                phonetic_uk: None,
                prompt: "discussion".to_string(),
                accepted_meanings: vec!["discussion".to_string()],
                example_sentence: None,
                example_translation: None,
                choices: Some(vec![
                    ChoiceOption {
                        text: "debate".to_string(),
                        label: "A".to_string(),
                    },
                    ChoiceOption {
                        text: "".to_string(),
                        label: "B".to_string(),
                    },
                ]),
                correct_choice_label: Some("A".to_string()),
                question_index: 15,
                total_questions: 28,
            }).expect("serialize question")],
            "results": [],
            "currentIndex": 0,
            "questionTypeWeights": [{
                "questionType": "cnToEnChoice",
                "weight": 100
            }]
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_review", snapshot.to_string()),
        )
        .expect("insert invalid active snapshot");

        let error = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::Review,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect_err("empty start request must not restore empty choice text");

        assert!(matches!(error, StudyError::NotEnoughWords));
        let remaining_snapshot =
            study_repo::load_active_session_snapshot(&conn, &SessionMode::Review)
                .expect("load snapshot");
        assert!(remaining_snapshot.is_none());
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
        let second_response = second_question
            .correct_choice_label
            .clone()
            .unwrap_or_else(|| second_question.accepted_meanings[0].clone());

        let error = submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: second_question.question_id.clone(),
                response: second_response,
                response_time_ms: 100,
            },
        )
        .expect_err("non-current question submit should be rejected");

        assert!(matches!(error, super::StudyError::NoActiveSession));
    }

    #[test]
    fn submit_answer_is_idempotent_for_already_answered_question() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let payloads = vec![
            test_entry("dup_one", "debate", "杈╄"),
            test_entry("dup_two", "opinion", "瑙傜偣"),
        ];
        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: payloads
                    .iter()
                    .map(|payload| payload.source_id.clone())
                    .collect(),
                entry_payloads: payloads,
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnInput,
                    weight: 100,
                }],
            },
        )
        .expect("start session");

        let question_id = start.current_question.question_id.clone();
        let first = submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: question_id.clone(),
                response: start.current_question.accepted_meanings[0].clone(),
                response_time_ms: 100,
            },
        )
        .expect("first submit");
        let duplicate = submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: question_id.clone(),
                response: "wrong duplicate tap".to_string(),
                response_time_ms: 10,
            },
        )
        .expect("duplicate submit should be idempotent");

        assert_eq!(duplicate.result.question_id, first.result.question_id);
        assert_eq!(duplicate.result.user_response, first.result.user_response);
        assert_eq!(duplicate.progress.current, first.progress.current);
        assert_eq!(duplicate.answered_questions.len(), 1);
        assert!(first.current_question.is_some());
        assert!(duplicate.current_question.is_none());
    }

    #[test]
    fn answered_feed_is_windowed_and_older_history_can_be_loaded() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let payloads = (0..30)
            .map(|index| {
                test_entry(
                    &format!("window_entry_{index}"),
                    &format!("word{index}"),
                    &format!("meaning{index}"),
                )
            })
            .collect::<Vec<_>>();
        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: payloads
                    .iter()
                    .map(|payload| payload.source_id.clone())
                    .collect(),
                entry_payloads: payloads,
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnInput,
                    weight: 100,
                }],
            },
        )
        .expect("start windowed session");

        let session_id = start.session.session_id.clone();
        let mut current_question = start.current_question;
        let mut latest = None;
        for _ in 0..25 {
            let response = submit_study_answer(
                &conn,
                SubmitAnswerRequest {
                    question_id: current_question.question_id,
                    response: current_question.accepted_meanings[0].clone(),
                    response_time_ms: 100,
                },
            )
            .expect("submit answer");
            current_question = response
                .current_question
                .clone()
                .expect("session should not be complete");
            latest = Some(response);
        }

        let latest = latest.expect("at least one response");
        assert_eq!(latest.progress.current, 26);
        assert_eq!(latest.answered_questions.len(), super::ANSWERED_FEED_WINDOW);
        assert_eq!(latest.answered_questions[0].question.question_index, 5);

        let older = super::get_study_session_answered_history(&session_id, 5, 5)
            .expect("load older history");
        assert_eq!(older.len(), 5);
        assert_eq!(older[0].question.question_index, 0);
        assert_eq!(older[4].question.question_index, 4);
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
        conn.execute_batch(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'final-dispute-test', 'ready');
             INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma, part_of_speech)
             VALUES (1, 1, 'alpha', 'alpha', 'alpha', 'n');",
        )
        .expect("seed entry");

        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: vec!["1".to_string()],
                entry_payloads: vec![StartSessionEntryPayload {
                    source_id: "1".to_string(),
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

        let dispute = accept_disputed_meaning(
            &conn,
            AcceptDisputedMeaningRequest {
                question_id: start.current_question.question_id.clone(),
                submitted_answer: "accepted final meaning".to_string(),
            },
        )
        .expect("completed final question can still accept dispute");
        assert_eq!(dispute.result.outcome, AnswerOutcome::FuzzyCorrect);
        assert_eq!(dispute.result.correct_answer, "accepted final meaning");
        assert_eq!(
            dispute
                .answered_questions
                .last()
                .expect("answered final question remains in feed")
                .result
                .outcome,
            AnswerOutcome::FuzzyCorrect
        );

        let stored_outcome: String = conn
            .query_row(
                "SELECT outcome FROM study_results WHERE question_id = ?1",
                [&start.current_question.question_id],
                |row| row.get(0),
            )
            .expect("stored completed result outcome");
        assert_eq!(stored_outcome, "\"fuzzyCorrect\"");

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
    fn completed_new_word_does_not_leak_feed_or_progress_into_review() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let new_word = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::NewWord,
                wordbook_id: None,
                entry_source_ids: vec!["new_alpha".to_string()],
                entry_payloads: vec![test_entry("new_alpha", "alpha", "first")],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("start new word");
        assert_eq!(new_word.progress.current, 1);
        assert_eq!(new_word.progress.total, 4);

        let mut current_question = new_word.current_question;
        for _ in 0..4 {
            let response = submit_study_answer(
                &conn,
                SubmitAnswerRequest {
                    question_id: current_question.question_id.clone(),
                    response: current_question
                        .correct_choice_label
                        .clone()
                        .unwrap_or_else(|| current_question.accepted_meanings[0].clone()),
                    response_time_ms: 100,
                },
            )
            .expect("submit new-word question");
            if response.is_complete {
                break;
            }
            current_question = response
                .current_question
                .expect("new-word session should continue until complete");
        }

        let new_word_snapshot =
            study_repo::load_active_session_snapshot(&conn, &SessionMode::NewWord)
                .expect("load new-word snapshot after completion");
        assert!(new_word_snapshot.is_none());

        let review = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::Review,
                wordbook_id: None,
                entry_source_ids: vec!["review_beta".to_string(), "review_gamma".to_string()],
                entry_payloads: vec![
                    test_entry("review_beta", "beta", "second"),
                    test_entry("review_gamma", "gamma", "third"),
                ],
                distractor_payloads: Vec::new(),
                question_type_weights: vec![QuestionTypeWeight {
                    question_type: QuestionType::EnToCnInput,
                    weight: 100,
                }],
            },
        )
        .expect("start review after new word completion");

        assert_eq!(review.progress.current, 1);
        assert_eq!(review.progress.total, 2);
        assert!(review.answered_questions.is_empty());
        assert!(review
            .current_question
            .entry_source_id
            .starts_with("review_"));
        assert_ne!(review.current_question.entry_source_id, "new_alpha");
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
                distractor_payloads: vec![
                    test_entry("distractor_one", "calm", "calm meaning"),
                    test_entry("distractor_two", "quiet", "quiet meaning"),
                    test_entry("distractor_three", "portable", "portable meaning"),
                ],
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
        let wrong_label = if wrong_label == correct_label {
            start
                .current_question
                .choices
                .as_ref()
                .and_then(|choices| choices.iter().find(|choice| choice.label != correct_label))
                .map(|choice| choice.label.clone())
                .expect("a distinct wrong choice exists")
        } else {
            wrong_label
        };
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
        let current = pruned.current_question.unwrap();
        assert_ne!(
            current.entry_source_id,
            start.current_question.entry_source_id
        );
        assert_eq!(current.question_index, 0);
        assert_eq!(current.total_questions, 4);
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

    #[test]
    fn mastered_answered_entry_keeps_answer_and_does_not_add_replacement() {
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
                    test_entry("alpha", "alpha", "alpha meaning"),
                    test_entry("beta", "beta", "beta meaning"),
                ],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("start session");
        let answered_source_id = start.current_question.entry_source_id.clone();
        let submitted = submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: start.current_question.question_id,
                response: start.current_question.accepted_meanings[0].clone(),
                response_time_ms: 100,
            },
        )
        .expect("submit answer");
        assert!(!submitted.is_complete);

        let marked = mark_study_entry_mastered(
            &conn,
            MarkStudyEntryMasteredRequest {
                entry_source_id: answered_source_id.clone(),
                reason: "mastered".to_string(),
            },
        )
        .expect("mark answered entry mastered");

        assert_eq!(marked.pruned_question_count, 0);
        assert!(!marked.is_complete);
        assert_eq!(marked.progress.total, 2);
        assert_eq!(marked.answered_questions.len(), 1);
        let final_question = marked.current_question.unwrap();
        assert_ne!(final_question.entry_source_id, answered_source_id);

        let final_submit = submit_study_answer(
            &conn,
            SubmitAnswerRequest {
                question_id: final_question.question_id,
                response: final_question.accepted_meanings[0].clone(),
                response_time_ms: 100,
            },
        )
        .expect("submit final answer");
        assert!(final_submit.is_complete);

        let final_marked = mark_study_entry_mastered(
            &conn,
            MarkStudyEntryMasteredRequest {
                entry_source_id: final_question.entry_source_id,
                reason: "mastered".to_string(),
            },
        )
        .expect("mark final answered entry mastered");
        assert_eq!(final_marked.pruned_question_count, 0);
        assert!(final_marked.is_complete);
        assert_eq!(final_marked.progress.total, 2);
        assert_eq!(final_marked.answered_questions.len(), 2);
    }

    #[test]
    fn mastered_unanswered_final_entry_uses_replacement_and_resumes_actual_plan() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        clear_all_active_sessions();

        let start = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::HighFrequency,
                wordbook_id: None,
                entry_source_ids: vec!["alpha".to_string()],
                entry_payloads: vec![test_entry("alpha", "alpha", "alpha meaning")],
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("start session");

        let replaced = mark_study_entry_mastered_with_replacements(
            &conn,
            MarkStudyEntryMasteredRequest {
                entry_source_id: start.current_question.entry_source_id,
                reason: "mastered".to_string(),
            },
            vec![test_entry("beta", "beta", "beta meaning")],
        )
        .expect("replace unanswered mastered entry");

        assert_eq!(replaced.pruned_question_count, 1);
        assert!(!replaced.is_complete);
        assert_eq!(replaced.progress.current, 1);
        assert_eq!(replaced.progress.total, 1);
        let replacement = replaced.current_question.expect("replacement question");
        assert_eq!(replacement.entry_source_id, "beta");
        assert_eq!(replacement.question_index, 0);
        assert_eq!(replacement.total_questions, 1);

        clear_all_active_sessions();
        let resumed = start_study_session(
            &conn,
            StartSessionRequest {
                mode: SessionMode::HighFrequency,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
                question_type_weights: Vec::new(),
            },
        )
        .expect("resume replacement plan");
        assert_eq!(resumed.current_question.entry_source_id, "beta");
        assert_eq!(resumed.progress.current, 1);
        assert_eq!(resumed.progress.total, 1);
    }
}
