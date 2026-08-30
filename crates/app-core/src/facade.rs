//! Facade layer for platform consumers.
//!
//! This module provides high-level APIs that both desktop and mobile
//! can use without dealing with internal crate organization.

pub mod bootstrap_facade;
pub mod study_facade;
pub mod today_facade;

pub use bootstrap_facade::{bootstrap, bootstrap_with_connection};
pub use study_facade::{
    accept_disputed_meaning, cancel_study_session, clear_all_active_sessions,
    complete_study_session, get_active_study_session, get_study_session_answered_history,
    mark_study_entry_mastered, mark_study_entry_mastered_with_replacements,
    recent_study_diagnostics, start_study_session, submit_study_answer,
};
pub use today_facade::{build_today_home_state, get_today_home_state};
