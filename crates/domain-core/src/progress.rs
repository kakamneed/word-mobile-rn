use word_domain_models::{AnswerOutcome, SessionProgress, SessionSummary, StudyResult};

#[derive(Debug, Clone, Default)]
pub struct StudyEntryState {
    pub entry_id: i64,
    pub review_count: i64,
    pub wrong_count: i64,
    pub fuzzy_correct_count: i64,
    pub last_seen_at: Option<String>,
    pub last_result: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub entry_id: i64,
    pub old_review_count: i64,
    pub new_review_count: i64,
    pub old_wrong_count: i64,
    pub new_wrong_count: i64,
}

pub fn apply_result(
    entry_id: i64,
    outcome: &AnswerOutcome,
    current_state: &StudyEntryState,
) -> StateTransition {
    let (new_review_count, new_wrong_count) = match outcome {
        AnswerOutcome::Correct | AnswerOutcome::FuzzyCorrect => {
            (current_state.review_count + 1, current_state.wrong_count)
        }
        AnswerOutcome::Incorrect => (
            current_state.review_count + 1,
            current_state.wrong_count + 1,
        ),
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

pub fn question_progress(answered_question_ids: &[String], total: u32) -> SessionProgress {
    let current = answered_question_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len()
        .min(total as usize) as u32;
    SessionProgress { current, total }
}

pub fn summarize_results(
    session_id: &str,
    results: &[StudyResult],
    total_questions: u32,
    completed_at: &str,
) -> SessionSummary {
    SessionSummary::from_results_with_total(
        session_id,
        results,
        total_questions,
        completed_at,
    )
}
