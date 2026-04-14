//! Bootstrap evaluation for app startup.

use rusqlite::Connection;

use word_storage_core::models::BootstrapState;
use word_storage_core::persistence;

use crate::platform::{PlatformError, PlatformRuntime};

/// Errors that can occur during bootstrap.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    #[error("Platform error: {0}")]
    Platform(#[from] PlatformError),
    #[error("Storage error: {0}")]
    Storage(String),
}

/// Evaluate the full bootstrap state for the application.
///
/// This is called on every app startup to determine the app's readiness state.
/// It checks directories, database, bundled snapshots, and onboarding status.
pub fn evaluate_bootstrap(
    runtime: &dyn PlatformRuntime,
    conn: &Connection,
) -> Result<BootstrapState, BootstrapError> {
    // Step 1: Environment initialization
    let dirs_ok = ensure_directories(runtime)?;

    if !dirs_ok {
        return Ok(BootstrapState {
            app_ready: false,
            first_run_required: false,
            database_status: "init_failed".to_string(),
            snapshot_status: "unknown".to_string(),
            connectivity_status: "unknown".to_string(),
            ai_config_status: "unknown".to_string(),
            settings_entry_available: false,
            blocking_reason: Some("Failed to create required directories.".to_string()),
        });
    }

    // Step 2: Database is already initialized (passed in)
    let database_status = "ready".to_string();

    // Step 3: Evaluate bundled snapshot
    let snapshot_status = evaluate_snapshot(runtime)?;

    // Step 4: Onboarding completion
    let first_run_required = !persistence::is_onboarding_completed(conn)
        .map_err(|e| BootstrapError::Storage(e.to_string()))?;

    // Step 5: Non-fatal status checks
    let connectivity_status = "offline".to_string(); // Phase 1: offline-first
    let ai_config_status = evaluate_ai_config(conn)?;

    // Determine blocking conditions
    let mut blocking_reason: Option<String> = None;
    let mut app_ready = true;

    if first_run_required && snapshot_status == "missing_required" {
        blocking_reason = Some("Bundled vocabulary snapshot is missing.".to_string());
        app_ready = false;
    }

    Ok(BootstrapState {
        app_ready,
        first_run_required,
        database_status,
        snapshot_status,
        connectivity_status,
        ai_config_status,
        settings_entry_available: app_ready,
        blocking_reason,
    })
}

fn ensure_directories(runtime: &dyn PlatformRuntime) -> Result<bool, BootstrapError> {
    let data_ok = runtime.app_data_dir().is_ok();
    let config_ok = runtime.app_config_dir().is_ok();
    let log_ok = runtime.app_log_dir().is_ok();
    Ok(data_ok && config_ok && log_ok)
}

fn evaluate_snapshot(runtime: &dyn PlatformRuntime) -> Result<String, BootstrapError> {
    match runtime.bundled_resource_path("vocab-snapshot/vocab-snapshot.jsonl") {
        Ok(path) => {
            if path.exists() {
                match std::fs::metadata(&path) {
                    Ok(meta) if meta.len() > 0 => Ok("ready".to_string()),
                    Ok(_) => Ok("empty".to_string()),
                    Err(_) => Ok("missing_required".to_string()),
                }
            } else {
                Ok("missing_required".to_string())
            }
        }
        Err(_) => Ok("missing_required".to_string()),
    }
}

fn evaluate_ai_config(conn: &Connection) -> Result<String, BootstrapError> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = 'ai_config'",
            [],
            |row| row.get(0),
        )
        .ok();

    Ok(match value {
        Some(v) if !v.is_empty() && v != "{}" => "configured".to_string(),
        _ => "missing".to_string(),
    })
}
