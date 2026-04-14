//! Facade layer for platform consumers.
//!
//! This module provides high-level APIs that both desktop and mobile
//! can use without dealing with internal crate organization.

pub mod bootstrap_facade;
pub mod study_facade;
pub mod today_facade;

pub use bootstrap_facade::bootstrap;
pub use study_facade::{cancel_study_session, complete_study_session, start_study_session, submit_study_answer};
pub use today_facade::get_today_home_state;
