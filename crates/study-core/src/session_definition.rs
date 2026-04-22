//! Session definition and mode rules for study sessions.

use word_storage_core::models::{QuestionType, SessionMode};

/// Session boundary rules for each of the four study modes.
#[derive(Debug, Clone)]
pub struct SessionDefinition {
    pub mode: SessionMode,
    pub rules: ModeRules,
}

/// Behavioral rules for a specific session mode.
#[derive(Debug, Clone)]
pub struct ModeRules {
    pub question_types: Vec<QuestionType>,
    pub loops_all_types_per_word: bool,
    pub batch_only: bool,
    pub from_wrong_pool: bool,
    pub word_selection_description: String,
}

impl SessionDefinition {
    /// Create the session definition for a given mode.
    pub fn for_mode(mode: SessionMode) -> Self {
        let rules = match &mode {
            SessionMode::NewWord => ModeRules {
                question_types: QuestionType::all_four(),
                loops_all_types_per_word: true,
                batch_only: true,
                from_wrong_pool: false,
                word_selection_description: String::from(
                    "Words are drawn from the day's new-word learning plan.",
                ),
            },
            SessionMode::Review => ModeRules {
                question_types: QuestionType::all_four(),
                loops_all_types_per_word: true,
                batch_only: false,
                from_wrong_pool: false,
                word_selection_description: String::from(
                    "Words are drawn from already-learned vocabulary.",
                ),
            },
            SessionMode::MixedTest => ModeRules {
                question_types: vec![
                    QuestionType::EnToCnChoice,
                    QuestionType::CnToEnChoice,
                    QuestionType::EnToCnInput,
                ],
                loops_all_types_per_word: false,
                batch_only: false,
                from_wrong_pool: false,
                word_selection_description: String::from(
                    "Words are drawn from the full learned vocabulary pool.",
                ),
            },
            SessionMode::WrongWordReinforcement => ModeRules {
                question_types: vec![
                    QuestionType::EnToCnChoice,
                    QuestionType::CnToEnChoice,
                    QuestionType::EnToCnInput,
                ],
                loops_all_types_per_word: false,
                batch_only: false,
                from_wrong_pool: true,
                word_selection_description: String::from(
                    "Words are drawn from the wrong-word pool.",
                ),
            },
            SessionMode::RootAffix => ModeRules {
                question_types: vec![
                    QuestionType::GlossToRootInput,
                    QuestionType::RootToGlossInput,
                ],
                loops_all_types_per_word: false,
                batch_only: false,
                from_wrong_pool: false,
                word_selection_description: String::from(
                    "Items are drawn from the scoped root/affix pool.",
                ),
            },
        };

        SessionDefinition { mode, rules }
    }
}
