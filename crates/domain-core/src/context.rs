use serde::{Deserialize, Serialize};

/// All time and ordering inputs required by exported calculations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainContext {
    pub now_utc: String,
    pub local_day: String,
    pub session_id: String,
    pub ordering_seed: String,
}
