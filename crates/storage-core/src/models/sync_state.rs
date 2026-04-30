use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOutboxStatus {
    Pending,
    InFlight,
    Succeeded,
    RetryableFailure,
    DeadLettered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutboxItem {
    pub id: i64,
    pub domain: String,
    pub payload_json: String,
    pub idempotency_key: String,
    pub created_at: String,
    pub attempt_count: i64,
    pub last_attempt_at: Option<String>,
    pub status: SyncOutboxStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCursorState {
    pub user_id: Option<String>,
    pub device_id: String,
    pub last_pushed_at: Option<String>,
    pub last_pulled_cursor: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDeadLetter {
    pub id: i64,
    pub domain: String,
    pub payload_json: String,
    pub idempotency_key: String,
    pub failure_code: String,
    pub failure_message: String,
    pub created_at: String,
    pub last_attempt_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncDomainPendingCount {
    pub domain: String,
    pub pending_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub sync_enabled: bool,
    pub transport_configured: bool,
    pub account_sync_state: String,
    pub pending_count: i64,
    pub last_sync_succeeded_at: Option<String>,
    pub last_sync_error_code: Option<String>,
    pub domains_pending: Vec<SyncDomainPendingCount>,
}
