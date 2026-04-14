use serde::{Deserialize, Serialize};

/// A plan template that defines daily study targets and growth rules.
///
/// Users edit this template on a single page. The template is the
/// "future-facing" definition that daily snapshots are generated from.
/// The template itself is NOT today's execution state -- that is the
/// DailySnapshot (see daily_snapshot.rs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanTemplate {
    pub id: Option<i64>,
    pub name: String,
    pub active_wordbook_ids: Vec<i64>,
    pub new_words_per_day: i64,
    pub review_words_per_day: i64,
    pub mixed_test_per_day: i64,
    pub wrong_word_test_per_day: i64,
    /// N in "every N days increase by X"
    pub growth_interval_days: i64,
    /// X in "every N days increase by X"
    pub growth_increment: i64,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl PlanTemplate {
    /// Serialize active_wordbook_ids to a JSON string for SQLite storage.
    pub fn to_json_wordbooks(&self) -> String {
        serde_json::to_string(&self.active_wordbook_ids).unwrap_or_else(|_| "[]".to_string())
    }

    /// Deserialize a JSON string from SQLite into a Vec<i64>.
    pub fn from_json_wordbooks(s: &str) -> Vec<i64> {
        serde_json::from_str(s).unwrap_or_default()
    }

    /// Returns a new PlanTemplate with sensible defaults.
    pub fn default_template() -> Self {
        Self {
            id: None,
            name: String::new(),
            active_wordbook_ids: Vec::new(),
            new_words_per_day: 20,
            review_words_per_day: 30,
            mixed_test_per_day: 10,
            wrong_word_test_per_day: 5,
            growth_interval_days: 7,
            growth_increment: 5,
            is_active: false,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
}
