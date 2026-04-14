use serde::{Deserialize, Serialize};

/// The user's submitted answer for a single question.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyAnswer {
    /// The question_id this answer corresponds to.
    pub question_id: String,

    /// For choice-type questions: the label of the selected option (e.g., "B").
    /// For input-type questions: the free-text input from the user.
    pub response: String,

    /// How many milliseconds the user spent before submitting this answer.
    pub response_time_ms: u64,
}

/// Graded outcome for a single answer evaluation.
///
/// This is the authoritative result enum used throughout the study engine.
/// It supports the graded semantics required by D-14 and the wrong-word
/// priority rules from D-15 through D-18.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AnswerOutcome {
    /// The answer is a clear, unambiguous match with the accepted meaning.
    Correct,

    /// The answer is a match after normalization but has non-trivial
    /// surface-level differences (e.g., minor wording variation, missing
    /// optional modifiers). Two fuzzy_correct results can be treated as
    /// roughly equivalent to one correct for scheduling purposes (D-18).
    FuzzyCorrect,

    /// The answer does not match any accepted meaning, even after normalization.
    Incorrect,

    /// The user explicitly skipped this question (e.g., pressed a skip button
    /// or left the input empty when submission was forced).
    Skipped,
}

impl AnswerOutcome {
    /// Whether this outcome counts as a positive signal for scheduling.
    /// Correct is full positive; FuzzyCorrect is partial positive.
    pub fn is_positive(&self) -> bool {
        matches!(self, AnswerOutcome::Correct | AnswerOutcome::FuzzyCorrect)
    }

    /// Whether this outcome indicates the user clearly does not know the word.
    pub fn is_clear_failure(&self) -> bool {
        matches!(self, AnswerOutcome::Incorrect | AnswerOutcome::Skipped)
    }

    /// Weight for wrong-word priority calculations.
    /// Higher values indicate stronger negative signal.
    /// - Incorrect/Skipped: strongly negative, word enters wrong-word pool (D-16).
    /// - FuzzyCorrect: mildly negative, handled lightly (D-17).
    /// - Correct: no penalty.
    pub fn wrong_word_weight(&self) -> f64 {
        match self {
            AnswerOutcome::Incorrect => 2.0,
            AnswerOutcome::Skipped => 2.5,
            AnswerOutcome::FuzzyCorrect => 0.5,
            AnswerOutcome::Correct => 0.0,
        }
    }

    /// Whether this outcome should enter the word into the wrong-word pool.
    pub fn enters_wrong_pool(&self) -> bool {
        matches!(self, AnswerOutcome::Incorrect | AnswerOutcome::Skipped)
    }
}
