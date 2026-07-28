use word_domain_models::{SessionProgress, SessionSummary, StudyResult};

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
