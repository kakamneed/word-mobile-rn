//! Today home state model.

use serde::{Deserialize, Serialize};

/// Full state for the today home dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayHomeState {
    pub today_date: String,
    pub active_plan: Option<PlanSummary>,
    pub today_snapshot: Option<DailySnapshot>,
    pub wordbooks: Vec<WordbookSummary>,
    pub daily_progress: DailyProgress,
}

/// Summary of a plan template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub id: i64,
    pub name: String,
    pub new_words_per_day: i64,
    pub review_words_per_day: i64,
    pub mixed_test_per_day: i64,
    pub wrong_word_test_per_day: i64,
    pub growth_interval_days: i64,
    pub growth_increment: i64,
}

/// Daily snapshot for a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySnapshot {
    pub date: String,
    pub new_words_target: u32,
    pub new_words_completed: u32,
    pub review_words_target: u32,
    pub review_words_completed: u32,
    pub mixed_test_target: u32,
    pub mixed_test_completed: u32,
    pub wrong_word_test_target: u32,
    pub wrong_word_test_completed: u32,
}

/// Summary of a wordbook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordbookSummary {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub category: String,
    pub total_entries: i64,
    pub is_active: bool,
}

/// Daily progress summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyProgress {
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub next_recommended_action: String,
}
