use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::DomainContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrongWordProjectionInput {
    pub entry_source_id: String,
    pub word: String,
    pub error_count: u64,
    #[serde(default)]
    pub correct_count: u64,
    #[serde(default)]
    pub consecutive_correct: u64,
    pub last_wrong_at: String,
    #[serde(default)]
    pub correct_since_last_wrong: u64,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrongWordProjection {
    pub entry_source_id: String,
    pub word: String,
    pub error_count: u64,
    pub correct_count: u64,
    pub consecutive_correct: u64,
    pub last_wrong_at: String,
    pub correct_since_last_wrong: u64,
    pub error_rate: f64,
    pub priority_score: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportHistoryInput {
    pub date: String,
    pub mode: String,
    pub summary: ReportHistorySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportHistorySummary {
    pub total_questions: u64,
    pub correct_count: u64,
    pub total_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDay {
    pub date: String,
    pub total_questions: u64,
    pub correct_count: u64,
    pub accuracy_percent: f64,
    pub study_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportModeSummary {
    pub mode: String,
    pub total_questions: u64,
    pub correct_count: u64,
    pub accuracy_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreakInfo {
    pub current_streak: u64,
    pub longest_streak: u64,
    pub last_study_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportProjection {
    pub total_study_days: u64,
    pub total_words_learned: u64,
    pub total_questions_answered: u64,
    pub overall_accuracy: f64,
    pub streak_info: StreakInfo,
    pub mode_breakdown: Vec<ReportModeSummary>,
    pub last7_days: Vec<ReportDay>,
    pub daily_series: Vec<ReportDay>,
    pub mode_series: BTreeMap<String, Vec<ReportDay>>,
}

fn default_true() -> bool {
    true
}

pub fn project_wrong_words(
    _context: &DomainContext,
    _entries: &[WrongWordProjectionInput],
    _filter: &str,
) -> Vec<WrongWordProjection> {
    panic!("wrong-word projection fixture not implemented")
}

pub fn project_report(
    _context: &DomainContext,
    _history: &[ReportHistoryInput],
    _learned_count: u64,
) -> ReportProjection {
    panic!("report projection fixture not implemented")
}

