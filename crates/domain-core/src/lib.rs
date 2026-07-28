//! Deterministic learning rules shared by native and WASM callers.

pub mod context;
pub mod progress;
pub mod projections;
pub mod study;

pub use context::DomainContext;
pub use progress::{
    apply_result, question_progress, summarize_results, StateTransition, StudyEntryState,
};
pub use projections::*;
pub use study::{session_definition, AnswerEvaluator, DomainModeRules, QuestionBuilder, WordForQuestion};
pub use word_domain_models::*;
