use serde::{Deserialize, Serialize};

/// Wrong-word pool state for a vocabulary entry.
///
/// Represents membership in the wrong-word pool and the reinforcement
/// signals that drive elevated reappearance probability. Clear errors
/// produce stronger reinforcement than ambiguous/partial outcomes (D-16, D-17).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrongWordState {
    /// The vocabulary entry this wrong-word state belongs to.
    pub entry_id: i64,

    /// Total number of incorrect (clear error) outcomes for this entry.
    /// Does not include fuzzy_correct or skipped outcomes.
    pub error_count: i64,

    /// ISO-8601 timestamp of the most recent incorrect outcome.
    pub last_wrong_at: String,

    /// Composite priority score (0.0+) that drives reappearance ordering.
    ///
    /// Higher values mean the word should appear sooner. The score
    /// accounts for error frequency, recency, and instability signals.
    /// This is NOT a full SRS difficulty parameter -- it is a pragmatic
    /// weighted priority for wrong-word pool ordering (D-24).
    pub priority_score: f64,

    /// Whether this entry is currently an active wrong-word pool member.
    ///
    /// Entries can be deactivated after sustained correct performance,
    /// reducing pool size without losing historical data.
    pub is_active: bool,
}

impl WrongWordState {
    /// Creates a fresh wrong-word state for an entry entering the pool.
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
