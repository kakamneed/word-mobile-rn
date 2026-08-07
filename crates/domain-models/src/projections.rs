use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayHomeState {
    pub today_date: String,
    pub active_plan: Option<PlanSummary>,
    pub today_snapshot: Option<DailySnapshot>,
    pub wordbooks: Vec<WordbookSummary>,
    pub daily_progress: DailyProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub id: i64,
    pub name: String,
    pub new_words_per_day: i64,
    pub review_words_per_day: i64,
    pub mixed_test_per_day: i64,
    pub wrong_word_test_per_day: i64,
    #[serde(default)]
    pub high_frequency_per_day: i64,
    pub root_affix_per_day: Option<i64>,
    pub growth_interval_days: i64,
    pub growth_increment: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_type_weights_by_mode: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailySnapshot {
    pub date: String,
    pub new_words_target: u32,
    pub new_words_base_target: Option<u32>,
    pub new_words_carryover_target: Option<u32>,
    pub new_words_completed: u32,
    pub review_words_target: u32,
    pub review_words_base_target: Option<u32>,
    pub review_words_carryover_target: Option<u32>,
    pub review_words_completed: u32,
    pub mixed_test_target: u32,
    pub mixed_test_base_target: Option<u32>,
    pub mixed_test_carryover_target: Option<u32>,
    pub mixed_test_completed: u32,
    pub wrong_word_test_target: u32,
    pub wrong_word_test_base_target: Option<u32>,
    pub wrong_word_test_carryover_target: Option<u32>,
    pub wrong_word_test_completed: u32,
    pub high_frequency_target: u32,
    pub high_frequency_base_target: Option<u32>,
    pub high_frequency_carryover_target: Option<u32>,
    pub high_frequency_completed: u32,
    pub root_affix_target: Option<u32>,
    pub root_affix_base_target: Option<u32>,
    pub root_affix_carryover_target: Option<u32>,
    pub root_affix_completed: Option<u32>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyProgress {
    pub total_tasks: u32,
    pub completed_tasks: u32,
    pub next_recommended_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayHomeStateSeed {
    pub today_date: String,
    pub active_plan: Option<PlanSummary>,
    pub wordbooks: Vec<WordbookSummary>,
    pub completions: TodayCompletionSeed,
    #[serde(default)]
    pub targets: TodayTargetSeed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayCompletionSeed {
    pub new_words_completed: u32,
    pub review_words_completed: u32,
    pub mixed_test_completed: u32,
    pub wrong_word_test_completed: u32,
    #[serde(default)]
    pub high_frequency_completed: u32,
    pub root_affix_completed: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayTargetSeed {
    pub new_words_target: Option<u32>,
    pub new_words_base_target: Option<u32>,
    pub new_words_carryover_target: Option<u32>,
    pub review_words_target: Option<u32>,
    pub review_words_base_target: Option<u32>,
    pub review_words_carryover_target: Option<u32>,
    pub mixed_test_target: Option<u32>,
    pub mixed_test_base_target: Option<u32>,
    pub mixed_test_carryover_target: Option<u32>,
    pub wrong_word_test_target: Option<u32>,
    pub wrong_word_test_base_target: Option<u32>,
    pub wrong_word_test_carryover_target: Option<u32>,
    pub high_frequency_target: Option<u32>,
    pub high_frequency_base_target: Option<u32>,
    pub high_frequency_carryover_target: Option<u32>,
    pub root_affix_target: Option<u32>,
    pub root_affix_base_target: Option<u32>,
    pub root_affix_carryover_target: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrongWordState {
    pub entry_id: i64,
    pub error_count: i64,
    pub last_wrong_at: String,
    pub priority_score: f64,
    pub is_active: bool,
}

impl WrongWordState {
    pub fn new(entry_id: i64) -> Self {
        Self {
            entry_id,
            error_count: 0,
            last_wrong_at: String::new(),
            priority_score: 0.0,
            is_active: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Phase6QuestionPlanRepair {
    pub removed_unanswered_question_ids: Vec<String>,
    pub preserved_unanswered_question_ids: Vec<String>,
    pub replenished_entry_source_ids: Vec<String>,
    pub shortage_questions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Phase6SkipEvidence {
    pub accepted_answer_created: bool,
    pub outcome: String,
    pub reveals_canonical_answer: bool,
    pub updates_wrong_words_and_reports: bool,
    pub requires_explicit_next: bool,
}
