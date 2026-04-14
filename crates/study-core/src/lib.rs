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
