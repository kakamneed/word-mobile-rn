//! Study session request/response DTOs.

use serde::{Deserialize, Serialize};

use crate::models::{AnswerOutcome, QuestionType, SessionMode, SessionSummary, StudyQuestion, StudyResult, StudySession};

/// Request to start a study session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionRequest {
    pub mode: SessionMode,
    pub wordbook_id: Option<i64>,
    pub entry_source_ids: Vec<String>,
}

/// Response from starting a study session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionResponse {
    pub session: StudySession,
    pub current_question: StudyQuestion,
    pub progress: SessionProgress,
}

/// Request to submit an answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAnswerRequest {
    pub question_id: String,
    pub response: String,
    pub response_time_ms: u64,
}

/// Response from submitting an answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAnswerResponse {
    pub result: StudyResult,
    pub is_complete: bool,
    pub current_question: Option<StudyQuestion>,
    pub summary: Option<SessionSummary>,
    pub next_action: Option<String>,
    pub progress: SessionProgress,
}

/// Response from completing a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSessionResponse {
    pub summary: SessionSummary,
    pub next_action: String,
}

/// Session progress indicator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProgress {
    pub current: u32,
    pub total: u32,
}
