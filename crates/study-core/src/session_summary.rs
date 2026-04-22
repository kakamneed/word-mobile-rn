//! Session summary generation.

use word_storage_core::models::{SessionMode, SessionSummary, StudyResult, StudySession};

/// Generates session summaries and next-action recommendations.
pub struct SessionSummaryService;

impl SessionSummaryService {
    /// Generate the end-of-session summary from results.
    pub fn build_summary(
        session: &StudySession,
        results: &[StudyResult],
        completed_at: &str,
    ) -> SessionSummary {
        SessionSummary::from_results(&session.session_id, results, completed_at)
    }

    /// Determine the recommended next action after a session.
    pub fn next_action(summary: &SessionSummary, mode: &SessionMode) -> String {
        if summary.wrong_word_count > 0 {
            format!(
                "Wrong word reinforcement ({} words)",
                summary.wrong_word_count
            )
        } else if summary.accuracy_percent < 80.0 {
            "Review fuzzy answers".to_string()
        } else if summary.accuracy_percent < 100.0 {
            "Continue with next batch".to_string()
        } else {
            match mode {
                SessionMode::NewWord => "Continue with next batch".to_string(),
                SessionMode::Review => "Review complete".to_string(),
                SessionMode::MixedTest => "Check wrong words".to_string(),
                SessionMode::WrongWordReinforcement => "Continue with new words".to_string(),
                SessionMode::RootAffix => "Continue with review".to_string(),
            }
        }
    }
}
