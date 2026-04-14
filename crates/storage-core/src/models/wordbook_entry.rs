use serde::{Deserialize, Serialize};

/// Links wordbooks to vocabulary entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordbookEntry {
    pub id: Option<i64>,
    pub wordbook_id: i64,
    pub entry_id: i64,
    pub sort_order: i64,
}
