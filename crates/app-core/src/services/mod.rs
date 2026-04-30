//! Domain services extracted from the bridge layer.
//!
//! These modules contain business logic previously embedded in
//! `platform-mobile/bridge.rs`. Moving them here makes them testable
//! without depending on the cdylib bridge.

pub mod reports_service;
pub mod wrong_words_service;
