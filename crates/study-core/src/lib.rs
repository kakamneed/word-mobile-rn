//! Study session domain logic for Word Mobile.
//!
//! This crate provides pure domain logic for study sessions including:
//! - Question generation
//! - Answer evaluation
//! - Session state transitions
//! - Session summaries
//!
//! This crate has no persistence or platform dependencies.

pub mod answer_evaluator;
pub mod question_builder;
pub mod session_definition;
pub mod session_summary;
pub mod state_transition;

pub use answer_evaluator::AnswerEvaluator;
pub use question_builder::{QuestionBuilder, WordForQuestion};
pub use session_definition::SessionDefinition;
pub use session_summary::SessionSummaryService;

#[cfg(test)]
mod domain_compatibility_tests {
    use super::{AnswerEvaluator, SessionDefinition, SessionSummaryService};
    use word_storage_core::models::{
        QuestionType, SessionMode, StudyAnswer, StudyQuestion, StudySession,
    };

    #[test]
    fn answer_evaluator_matches_direct_domain_call() {
        let question = StudyQuestion {
            question_id: "q1".to_string(),
            question_type: QuestionType::EnToCnInput,
            entry_source_id: "entry-alpha".to_string(),
            word: "alpha".to_string(),
            part_of_speech: None,
            phonetic_us: None,
            phonetic_uk: None,
            prompt: "alpha".to_string(),
            accepted_meanings: vec!["alpha meaning".to_string()],
            example_sentence: None,
            example_translation: None,
            choices: None,
            correct_choice_label: None,
            question_index: 0,
            total_questions: 1,
        };
        let answer = StudyAnswer {
            question_id: "q1".to_string(),
            response: "alpha meaning".to_string(),
            response_time_ms: 100,
        };
        let native = AnswerEvaluator::evaluate(&question, &answer, "2026-07-28T00:00:00Z");
        let direct = word_domain_core::AnswerEvaluator::evaluate(
            &question,
            &answer,
            "2026-07-28T00:00:00Z",
        );
        assert_eq!(serde_json::to_value(native).unwrap(), serde_json::to_value(direct).unwrap());
    }

    #[test]
    fn review_definition_remains_single_question_while_newword_loops_four() {
        let native_new = SessionDefinition::for_mode(SessionMode::NewWord);
        let direct_new = word_domain_core::session_definition(SessionMode::NewWord);
        let native_review = SessionDefinition::for_mode(SessionMode::Review);
        let direct_review = word_domain_core::session_definition(SessionMode::Review);

        assert!(native_new.rules.loops_all_types_per_word);
        assert_eq!(native_new.rules.loops_all_types_per_word, direct_new.loops_all_types_per_word);
        assert!(!native_review.rules.loops_all_types_per_word);
        assert_eq!(native_review.rules.loops_all_types_per_word, direct_review.loops_all_types_per_word);
    }

    #[test]
    fn session_summary_matches_direct_question_unit_summary() {
        let session = StudySession {
            session_id: "summary-equivalence".to_string(),
            mode: SessionMode::Review,
            total_words: 1,
            wordbook_id: None,
            started_at: "2026-07-28T00:00:00Z".to_string(),
        };
        let native = SessionSummaryService::build_summary_with_total(
            &session,
            &[],
            1,
            "2026-07-28T00:01:00Z",
        );
        let direct = word_domain_core::summarize_results(
            &session.session_id,
            &[],
            1,
            "2026-07-28T00:01:00Z",
        );
        assert_eq!(serde_json::to_value(native).unwrap(), serde_json::to_value(direct).unwrap());
    }
}
