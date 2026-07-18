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
pub use standardized_entry::{EntryExample, MeaningZh, StandardizedEntry};
pub use study_answer::{AnswerOutcome, StudyAnswer};
pub use study_question::{ChoiceOption, QuestionType, StudyQuestion};
pub use study_requests::{
    AcceptDisputedMeaningRequest, AcceptDisputedMeaningResponse, AnsweredStudyQuestion,
    CompleteSessionResponse, MarkStudyEntryMasteredRequest, MarkStudyEntryMasteredResponse,
    QuestionTypeWeight, SessionProgress, StartSessionEntryPayload, StartSessionMeaningPayload,
    StartSessionRequest, StartSessionResponse, SubmitAnswerRequest, SubmitAnswerResponse,
};
pub use study_result::{SessionSummary, StudyResult};
pub use study_session::{SessionMode, StudySession};
pub use sync_state::{
    SyncCursorState, SyncDeadLetter, SyncDomainPendingCount, SyncOutboxItem, SyncOutboxStatus,
    SyncStatus,
};
pub use today_home_state::{
    DailyProgress, DailySnapshot, PlanSummary, TodayCompletionSeed, TodayHomeState,
    TodayHomeStateSeed, TodayTargetSeed, WordbookSummary,
};
pub use wordbook::Wordbook;
pub use wordbook_entry::WordbookEntry;
pub use wrong_word_state::WrongWordState;
