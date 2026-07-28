use word_domain_models::{SessionProgress, SessionSummary, StudyResult};

pub fn question_progress(answered_question_ids: &[String], total: u32) -> SessionProgress {
    let _ = answered_question_ids;
    SessionProgress { current: 0, total }
}

pub fn summarize_results(
    _session_id: &str,
    _results: &[StudyResult],
    _total_questions: u32,
    _completed_at: &str,
) -> SessionSummary {
    panic!("progress fixture not implemented")
}

