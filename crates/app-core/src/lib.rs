//! Application core for Word Mobile.
//!
//! This crate provides application lifecycle management and bootstrap orchestration
//! that is platform-agnostic. Platform-specific implementations (Tauri, React Native)
//! implement the `PlatformRuntime` trait.

pub mod bootstrap;
pub mod facade;
pub mod platform;
pub mod services;

pub use bootstrap::evaluate_bootstrap;
pub use facade::{
    accept_disputed_meaning, bootstrap, bootstrap_with_connection, build_today_home_state,
    cancel_study_session, clear_all_active_sessions, complete_study_session,
    get_active_study_session, get_study_session_answered_history, get_today_home_state,
    mark_study_entry_mastered, recent_study_diagnostics, start_study_session, submit_study_answer,
};
pub use platform::{PlatformError, PlatformRuntime};
