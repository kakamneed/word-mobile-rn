//! Native compatibility service for shared question-unit summaries.

use word_storage_core::models::{SessionMode, SessionSummary, StudyResult, StudySession};

pub struct SessionSummaryService;

impl SessionSummaryService {
    pub fn build_summary(
        session: &StudySession,
        results: &[StudyResult],
        completed_at: &str,
    ) -> SessionSummary {
        word_domain_core::summarize_results(
            &session.session_id,
            results,
            results.len() as u32,
            completed_at,
        )
    }

    pub fn build_summary_with_total(
        session: &StudySession,
        results: &[StudyResult],
        total_questions: u32,
        completed_at: &str,
    ) -> SessionSummary {
        word_domain_core::summarize_results(
            &session.session_id,
            results,
            total_questions,
            completed_at,
        )
    }

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
                SessionMode::HighFrequency => "Continue with high-frequency words".to_string(),
                SessionMode::RootAffix => "Continue with review".to_string(),
            }
        }
    }
}
