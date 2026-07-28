//! Native compatibility shape for shared session-mode rules.

use word_storage_core::models::{QuestionType, SessionMode};

#[derive(Debug, Clone)]
pub struct SessionDefinition {
    pub mode: SessionMode,
    pub rules: ModeRules,
}

#[derive(Debug, Clone)]
pub struct ModeRules {
    pub question_types: Vec<QuestionType>,
    pub loops_all_types_per_word: bool,
    pub batch_only: bool,
    pub from_wrong_pool: bool,
    pub word_selection_description: String,
}

impl SessionDefinition {
    pub fn for_mode(mode: SessionMode) -> Self {
        let shared = word_domain_core::session_definition(mode.clone());
        let word_selection_description = match mode {
            SessionMode::NewWord => "Words are drawn from the day's new-word learning plan.",
            SessionMode::Review => "Words are drawn from already-learned vocabulary.",
            SessionMode::MixedTest => "Words are drawn from the full learned vocabulary pool.",
            SessionMode::WrongWordReinforcement => "Words are drawn from the wrong-word pool.",
            SessionMode::RootAffix => "Items are drawn from the scoped root/affix pool.",
        };
        Self {
            mode,
            rules: ModeRules {
                question_types: shared.question_types,
                loops_all_types_per_word: shared.loops_all_types_per_word,
                batch_only: shared.batch_only,
                from_wrong_pool: shared.from_wrong_pool,
                word_selection_description: word_selection_description.to_string(),
            },
        }
    }
}
