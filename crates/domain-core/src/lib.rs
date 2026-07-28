//! Deterministic learning rules shared by native and WASM callers.

pub mod context;
pub mod progress;
pub mod study;

pub use context::DomainContext;
pub use progress::{question_progress, summarize_results};
pub use study::{QuestionBuilder, WordForQuestion};
pub use word_domain_models::*;

