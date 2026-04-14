use serde::{Deserialize, Serialize};

/// The four question types used in new-word learning and old-word review.
///
/// For new-word and review modes, each word goes through all four types
/// in order (D-07, D-08). Mixed test and wrong-word reinforcement may
/// use a subset or different selection strategy (D-09).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QuestionType {
    /// "看英文，选中文释义" -- See the English word, pick the correct
    /// Chinese meaning from multiple choices.
    EnToCnChoice,

    /// "看例句，选中文释义" -- See an example sentence containing the
    /// word, pick the correct Chinese meaning.
    ExampleToCnChoice,

    /// "看中文，选英文释义" -- See the Chinese meaning, pick the correct
    /// English word from multiple choices.
    CnToEnChoice,

    /// "看英文，输入中文释义" -- See the English word, type the Chinese
    /// meaning. Requires normalized matching evaluation (D-10, D-11).
    EnToCnInput,
}

impl QuestionType {
    /// Whether this question type requires free-text input from the user.
    pub fn is_input_type(&self) -> bool {
        matches!(self, QuestionType::EnToCnInput)
    }

    /// Whether this question type requires selecting from choices.
    pub fn is_choice_type(&self) -> bool {
        !self.is_input_type()
    }

    /// Returns the four question types in the locked loop order for
    /// new-word learning and old-word review (D-08).
    pub fn all_four() -> Vec<QuestionType> {
        vec![
            QuestionType::EnToCnChoice,
            QuestionType::ExampleToCnChoice,
            QuestionType::CnToEnChoice,
            QuestionType::EnToCnInput,
        ]
    }
}

/// A single choice option in a multiple-choice question.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceOption {
    /// The display text of this option.
    pub text: String,

    /// The index/label of this option (e.g., "A", "B", "C", "D").
    pub label: String,
}

/// A single question presented to the user during a study session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudyQuestion {
    /// Unique identifier for this question instance.
    pub question_id: String,

    /// The type of question.
    pub question_type: QuestionType,

    /// The word being tested. This is the stable source_id from StandardizedEntry.
    pub entry_source_id: String,

    /// The headword being tested.
    pub word: String,

    /// The prompt text shown to the user.
    /// - EnToCnChoice: the English word.
    /// - ExampleToCnChoice: the example sentence.
    /// - CnToEnChoice: the Chinese meaning.
    /// - EnToCnInput: the English word.
    pub prompt: String,

    /// All accepted Chinese meanings for the word (used for EnToCnInput
    /// evaluation and as the correct answer text for choice types).
    pub accepted_meanings: Vec<String>,

    /// The example sentence (if applicable, for ExampleToCnChoice).
    pub example_sentence: Option<String>,

    /// The Chinese translation of the example sentence (if applicable).
    pub example_translation: Option<String>,

    /// Available choices for multiple-choice questions.
    /// None for input-type questions.
    pub choices: Option<Vec<ChoiceOption>>,

    /// The label of the correct choice (e.g., "B"). None for input-type questions.
    pub correct_choice_label: Option<String>,

    /// Zero-based index of this question within the session's question sequence.
    pub question_index: u32,

    /// Total number of questions in this session.
    pub total_questions: u32,
}
