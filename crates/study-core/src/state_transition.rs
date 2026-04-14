//! Study state transitions.

use word_storage_core::models::AnswerOutcome;

/// Represents a study state transition result.
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub entry_id: i64,
    pub old_review_count: i64,
    pub new_review_count: i64,
    pub old_wrong_count: i64,
    pub new_wrong_count: i64,
}

/// Apply a study result to update state.
pub fn apply_result(entry_id: i64, outcome: &AnswerOutcome, current_state: &StudyEntryState) -> StateTransition {
    let (new_review_count, new_wrong_count) = match outcome {
        AnswerOutcome::Correct => (current_state.review_count + 1, current_state.wrong_count),
        AnswerOutcome::FuzzyCorrect => (current_state.review_count + 1, current_state.wrong_count),
        AnswerOutcome::Incorrect => (current_state.review_count + 1, current_state.wrong_count + 1),
        AnswerOutcome::Skipped => (current_state.review_count, current_state.wrong_count + 1),
    };

    StateTransition {
        entry_id,
        old_review_count: current_state.review_count,
        new_review_count,
        old_wrong_count: current_state.wrong_count,
        new_wrong_count,
    }
}

/// Current state of a study entry.
#[derive(Debug, Clone, Default)]
pub struct StudyEntryState {
    pub entry_id: i64,
    pub review_count: i64,
    pub wrong_count: i64,
    pub fuzzy_correct_count: i64,
    pub last_seen_at: Option<String>,
    pub last_result: Option<String>,
}
