use serde::{Deserialize, Serialize};

use crate::models::{
    QuestionType, SessionMode, SessionSummary, StudyQuestion, StudyResult, StudySession,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionMeaningPayload {
    pub pos: String,
    pub meaning_cn: String,
    #[serde(default)]
    pub meaning_en: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionEntryPayload {
    pub source_id: String,
    pub word: String,
    pub part_of_speech: Option<String>,
    #[serde(default)]
    pub frequency: f64,
    pub phonetic_us: Option<String>,
    pub phonetic_uk: Option<String>,
    #[serde(default)]
    pub meaning_details: Vec<StartSessionMeaningPayload>,
    pub meanings: Vec<String>,
    pub example_sentence: Option<String>,
    pub example_translation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuestionTypeWeight {
    pub question_type: QuestionType,
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionRequest {
    pub mode: SessionMode,
    pub wordbook_id: Option<i64>,
    pub entry_source_ids: Vec<String>,
    #[serde(default)]
    pub entry_payloads: Vec<StartSessionEntryPayload>,
    #[serde(default)]
    pub distractor_payloads: Vec<StartSessionEntryPayload>,
    #[serde(default)]
    pub question_type_weights: Vec<QuestionTypeWeight>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnsweredStudyQuestion {
    pub question: StudyQuestion,
    pub result: StudyResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionResponse {
    pub session: StudySession,
    pub current_question: StudyQuestion,
    pub progress: SessionProgress,
    #[serde(default)]
    pub answered_questions: Vec<AnsweredStudyQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAnswerRequest {
    pub question_id: String,
    pub response: String,
    pub response_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitAnswerResponse {
    pub result: StudyResult,
    pub is_complete: bool,
    pub current_question: Option<StudyQuestion>,
    pub summary: Option<SessionSummary>,
    pub next_action: Option<String>,
    pub progress: SessionProgress,
    #[serde(default)]
    pub answered_questions: Vec<AnsweredStudyQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkStudyEntryMasteredRequest {
    pub entry_source_id: String,
    #[serde(default = "default_mastered_reason")]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkStudyEntryMasteredResponse {
    pub entry_source_id: String,
    pub entry_id: Option<i64>,
    pub pruned_question_count: u32,
    pub is_complete: bool,
    pub current_question: Option<StudyQuestion>,
    pub summary: Option<SessionSummary>,
    pub next_action: Option<String>,
    pub progress: SessionProgress,
    #[serde(default)]
    pub answered_questions: Vec<AnsweredStudyQuestion>,
}

fn default_mastered_reason() -> String {
    "mastered".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSessionResponse {
    pub summary: SessionSummary,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionProgress {
    pub current: u32,
    pub total: u32,
}
