use serde::{Deserialize, Serialize};

use super::study_answer::AnswerOutcome;
use super::study_question::QuestionType;

/// The graded result of evaluating a single question answer.
///
/// This is the authoritative per-question record used for session summary
/// aggregation, wrong-word pool management, and scheduling decisions.
/// It preserves enough detail for chart-based follow-up reporting (D-23).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyResult {
    /// The question_id this result corresponds to.
    pub question_id: String,

    /// The entry_source_id of the word that was tested.
    pub entry_source_id: String,

    /// The type of question that was answered.
    pub question_type: QuestionType,

    /// The user's submitted response (choice label or text input).
    pub user_response: String,

    /// The normalized version of the user's response (after stripping
    /// brackets, whitespace, etc.). Useful for debugging.
    pub normalized_response: Option<String>,

    /// The accepted meaning(s) that were being matched against.
    /// For choice questions, this is the text of the correct choice.
    pub correct_answer: String,

    /// The graded outcome of this answer.
    pub outcome: AnswerOutcome,

    /// How many milliseconds the user spent answering.
    pub response_time_ms: u64,

    /// ISO 8601 timestamp of when this result was recorded.
    pub answered_at: String,
}

/// Per-session aggregate summary computed from a vector of StudyResult.
///
/// This is the concise end-of-session summary shown to the user (D-20, D-21),
/// plus the structured fields needed for later chart/report phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    /// The session_id this summary belongs to.
    pub session_id: String,

    /// Total number of questions answered.
    pub total_questions: u32,

    /// Number of questions answered correctly.
    pub correct_count: u32,

    /// Number of questions answered with fuzzy correct.
    pub fuzzy_correct_count: u32,

    /// Number of questions answered incorrectly.
    pub incorrect_count: u32,

    /// Number of questions skipped.
    pub skipped_count: u32,

    /// Total number of distinct words tested.
    pub total_words: u32,

    /// Number of distinct words with at least one incorrect or skipped result.
    pub wrong_word_count: u32,

    /// Accuracy as a percentage (0.0 to 100.0).
    /// Combines correct and fuzzy_correct as positive results.
    pub accuracy_percent: f64,

    /// Total wall-clock time for the session in milliseconds.
    pub total_time_ms: u64,

    /// ISO 8601 timestamp of when the session was completed.
    pub completed_at: String,
}

impl SessionSummary {
    /// Compute a SessionSummary from a list of results.
    pub fn from_results(session_id: &str, results: &[StudyResult], completed_at: &str) -> Self {
        let total_questions = results.len() as u32;
        let correct_count = results
            .iter()
            .filter(|r| r.outcome == AnswerOutcome::Correct)
            .count() as u32;
        let fuzzy_correct_count = results
            .iter()
            .filter(|r| r.outcome == AnswerOutcome::FuzzyCorrect)
            .count() as u32;
        let incorrect_count = results
            .iter()
            .filter(|r| r.outcome == AnswerOutcome::Incorrect)
            .count() as u32;
        let skipped_count = results
            .iter()
            .filter(|r| r.outcome == AnswerOutcome::Skipped)
            .count() as u32;

        let total_words = {
            let mut words: std::collections::HashSet<String> = std::collections::HashSet::new();
            for r in results {
                words.insert(r.entry_source_id.clone());
            }
            words.len() as u32
        };

        let wrong_word_count = {
            let mut wrong_words: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for r in results {
                if r.outcome.enters_wrong_pool() {
                    wrong_words.insert(r.entry_source_id.clone());
                }
            }
            wrong_words.len() as u32
        };

        let positive_count = (correct_count + fuzzy_correct_count) as f64;
        let accuracy_percent = if total_questions > 0 {
            (positive_count / total_questions as f64) * 100.0
        } else {
            0.0
        };

        let total_time_ms: u64 = results.iter().map(|r| r.response_time_ms).sum();

        SessionSummary {
            session_id: session_id.to_string(),
            total_questions,
            correct_count,
            fuzzy_correct_count,
            incorrect_count,
            skipped_count,
            total_words,
            wrong_word_count,
            accuracy_percent,
            total_time_ms,
            completed_at: completed_at.to_string(),
        }
    }
}
