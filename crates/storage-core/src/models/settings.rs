use serde::{Deserialize, Serialize};

/// A single key-value setting entry as stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingEntry {
    pub key: String,
    pub value_json: String,
    pub updated_at: String,
}

/// Baseline settings summary returned to the frontend for the settings page.
/// In Phase 1 this is a read-oriented shell with placeholders for future
/// model config and sync config sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsSummary {
    /// Whether an AI model configuration exists.
    pub ai_configured: bool,

    /// Whether a sync configuration exists.
    pub sync_configured: bool,

    /// Application version string.
    pub app_version: String,

    /// Schema version of the local database.
    pub schema_version: i64,

    /// Database file path for display purposes.
    pub database_path: String,
}
