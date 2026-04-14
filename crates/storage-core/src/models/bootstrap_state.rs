use serde::{Deserialize, Serialize};

/// Unified bootstrap contract returned to the frontend on every app start.
///
/// The frontend routing layer consumes this single structure to decide
/// whether the user should see the onboarding wizard, the dashboard, or
/// a blocking error page. No scattered page-level startup checks needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapState {
    /// Whether the application is ready for normal use.
    /// False only when a fatal condition is present.
    pub app_ready: bool,

    /// Whether the user has not yet completed first-run onboarding.
    pub first_run_required: bool,

    /// Status of the local SQLite database.
    /// "ready" when initialization succeeded, "init_failed" otherwise.
    pub database_status: String,

    /// Status of the bundled vocabulary snapshot.
    /// "ready" when snapshot resource exists, "missing_required" otherwise.
    pub snapshot_status: String,

    /// Network connectivity status. Non-fatal; "offline" should show a
    /// warning banner but must not block the main flow.
    pub connectivity_status: String,

    /// AI model configuration status. Non-fatal; "missing" should show a
    /// warning in settings but must not block the main flow.
    pub ai_config_status: String,

    /// Whether the settings entry should be visible to the user.
    /// True when the database is operational and the app shell can render.
    pub settings_entry_available: bool,

    /// If a fatal condition is present, this contains a human-readable
    /// reason explaining why the app cannot proceed. None when app_ready
    /// is true.
    pub blocking_reason: Option<String>,
}
