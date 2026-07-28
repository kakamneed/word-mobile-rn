//! Domain models for Word Mobile.
//!
//! All models are platform-agnostic and serializable for JSON contracts.

pub mod bootstrap_state;
pub mod exercise_vocab;
pub mod plan_template;
pub mod settings;
pub mod standardized_entry;
pub mod study_answer;
pub mod study_question;
pub mod study_requests;
pub mod study_result;
pub mod study_session;
pub mod sync_state;
pub mod today_home_state;
pub mod wordbook;
pub mod wordbook_entry;
pub mod wrong_word_state;

pub use bootstrap_state::BootstrapState;
pub use exercise_vocab::{
    ExerciseAnnotation, ExerciseAnnotationDraft, ExerciseArticle, ExerciseArticleDraft,
    ExerciseAttempt, ExerciseAttemptDraft, ExerciseVocabOccurrence, ExerciseVocabOccurrenceDraft,
    ExerciseVocabRelation, ExerciseWordMarkState,
};
pub use plan_template::PlanTemplate;
pub use settings::{SettingEntry, SettingsSummary};
pub use word_domain_models::{EntryExample, MeaningZh, StandardizedEntry};
pub use word_domain_models::{AnswerOutcome, StudyAnswer};
pub use word_domain_models::{ChoiceOption, QuestionType, StudyQuestion};
pub use word_domain_models::{
    AcceptDisputedMeaningRequest, AcceptDisputedMeaningResponse, AnsweredStudyQuestion,
    CompleteSessionResponse, MarkStudyEntryMasteredRequest, MarkStudyEntryMasteredResponse,
    QuestionTypeWeight, SessionProgress, StartSessionEntryPayload, StartSessionMeaningPayload,
    StartSessionRequest, StartSessionResponse, SubmitAnswerRequest, SubmitAnswerResponse,
};
pub use word_domain_models::{SessionSummary, StudyResult};
pub use word_domain_models::{SessionMode, StudySession};
pub use sync_state::{
    SyncCursorState, SyncDeadLetter, SyncDomainPendingCount, SyncOutboxItem, SyncOutboxStatus,
    SyncStatus,
};
pub use word_domain_models::{
    DailyProgress, DailySnapshot, PlanSummary, TodayCompletionSeed, TodayHomeState,
    TodayHomeStateSeed, TodayTargetSeed, WordbookSummary,
};
pub use wordbook::Wordbook;
pub use wordbook_entry::WordbookEntry;
pub use word_domain_models::WrongWordState;

#[cfg(test)]
mod canonical_reexport_tests {
    use super::StudyQuestion;

    fn require_same_type<T>(_: T, _: T) {}

    #[test]
    fn storage_study_question_is_the_canonical_type() {
        let fixture = include_str!(
            "../../../../fixtures/domain/v1/expected/study-resume-state.json"
        );
        let value: serde_json::Value = serde_json::from_str(fixture).expect("parse fixture");
        let canonical: word_domain_models::StudyQuestion =
            serde_json::from_value(value["currentQuestion"].clone()).expect("canonical DTO");
        let compatibility: StudyQuestion =
            serde_json::from_value(value["currentQuestion"].clone()).expect("storage facade DTO");

        require_same_type(canonical, compatibility);
    }
}
