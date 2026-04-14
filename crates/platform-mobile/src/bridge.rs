//! Native bridge exports for React Native.
//!
//! This module provides the FFI boundary between React Native (JavaScript)
//! and the shared Rust core. Functions here are exported to the native modules
//! on iOS and Android.

use std::sync::Mutex;

use once_cell::sync::Lazy;

use word_app_core::{
    bootstrap, bootstrap_with_connection, cancel_study_session, complete_study_session,
    get_today_home_state, start_study_session, submit_study_answer, PlatformRuntime,
};
use word_storage_core::persistence;

use crate::paths::MobilePaths;
use crate::runtime::MobileRuntime;

// Global runtime state
static MOBILE_RUNTIME: Lazy<Mutex<Option<MobileRuntime>>> = Lazy::new(|| Mutex::new(None));

/// Initialize the mobile runtime with platform-provided paths.
///
/// This should be called once during app startup before any other bridge functions.
pub fn initialize_mobile_runtime(
    app_data_dir: String,
    app_config_dir: String,
    app_cache_dir: String,
    bundle_resource_dir: String,
) -> Result<(), String> {
    let paths = MobilePaths::new(
        app_data_dir.into(),
        app_config_dir.into(),
        app_cache_dir.into(),
        bundle_resource_dir.into(),
    );

    let runtime = MobileRuntime::new(paths);

    let mut guard = MOBILE_RUNTIME
        .lock()
        .map_err(|_| "Failed to lock runtime")?;
    *guard = Some(runtime);

    Ok(())
}

/// Get the global runtime instance.
fn get_runtime() -> Result<impl Deref<Target = MobileRuntime>, String> {
    MOBILE_RUNTIME
        .lock()
        .map_err(|_| "Failed to lock runtime".to_string())
}

use std::ops::Deref;

// ============================================================================
// Bootstrap API
// ============================================================================

/// Get the bootstrap state.
///
/// Returns a JSON string containing BootstrapState.
pub fn get_bootstrap_state() -> Result<String, String> {
    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let state = bootstrap_with_connection(runtime, &conn)
        .map_err(|e| format!("Bootstrap failed: {}", e))?;

    serde_json::to_string(&state).map_err(|e| format!("JSON serialization failed: {}", e))
}

// ============================================================================
// Today Home API
// ============================================================================

/// Get the today home state.
///
/// Returns a JSON string containing TodayHomeState.
pub fn get_today_home_state() -> Result<String, String> {
    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let state = get_today_home_state(&conn).map_err(|e| format!("Failed to get today state: {}", e))?;

    serde_json::to_string(&state).map_err(|e| format!("JSON serialization failed: {}", e))
}

// ============================================================================
// Settings API
// ============================================================================

/// Get settings summary.
///
/// Returns a JSON string containing SettingsSummary.
pub fn get_settings() -> Result<String, String> {
    // Placeholder - would fetch from database
    let settings = serde_json::json!({
        "aiConfigured": false,
        "syncConfigured": false,
        "appVersion": "0.1.0",
        "schemaVersion": 7,
        "databasePath": ""
    });

    serde_json::to_string(&settings).map_err(|e| format!("JSON serialization failed: {}", e))
}

// ============================================================================
// Study Session API
// ============================================================================

use word_storage_core::models::{StartSessionRequest, SubmitAnswerRequest};

/// Start a study session.
///
/// Takes a JSON string containing StartSessionRequest.
/// Returns a JSON string containing StartSessionResponse.
pub fn start_study_session(request_json: String) -> Result<String, String> {
    let request: StartSessionRequest =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;

    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let response = start_study_session(&conn, request)
        .map_err(|e| format!("Failed to start session: {}", e))?;

    serde_json::to_string(&response).map_err(|e| format!("JSON serialization failed: {}", e))
}

/// Submit a study answer.
///
/// Takes a JSON string containing SubmitAnswerRequest.
/// Returns a JSON string containing SubmitAnswerResponse.
pub fn submit_study_answer(request_json: String) -> Result<String, String> {
    let request: SubmitAnswerRequest =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;

    let response = submit_study_answer(request)
        .map_err(|e| format!("Failed to submit answer: {}", e))?;

    serde_json::to_string(&response).map_err(|e| format!("JSON serialization failed: {}", e))
}

/// Complete the current study session.
///
/// Takes a session ID string.
/// Returns a JSON string containing CompleteSessionResponse.
pub fn complete_study_session(session_id: String) -> Result<String, String> {
    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let response = complete_study_session(&conn)
        .map_err(|e| format!("Failed to complete session: {}", e))?;

    serde_json::to_string(&response).map_err(|e| format!("JSON serialization failed: {}", e))
}

/// Cancel the current study session.
pub fn cancel_study_session() -> Result<(), String> {
    cancel_study_session().map_err(|e| format!("Failed to cancel session: {}", e))
}

// ============================================================================
// Platform-specific exports
// ============================================================================

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "ios")]
pub mod ios;
