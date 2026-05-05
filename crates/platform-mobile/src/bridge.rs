//! Native bridge exports for React Native.
//!
//! This module provides the FFI boundary between React Native (JavaScript)
//! and the shared Rust core. Functions here are exported to the native modules
//! on iOS and Android.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use once_cell::sync::Lazy;
use rusqlite::OptionalExtension;

use word_app_core::services::reports_service as reports_domain;
use word_app_core::services::wrong_words_service as wrong_words_domain;
use word_app_core::{
    bootstrap_with_connection as core_bootstrap_with_connection,
    build_today_home_state as core_build_today_home_state,
    cancel_study_session as core_cancel_study_session,
    clear_all_active_sessions as core_clear_all_active_sessions,
    complete_study_session as core_complete_study_session,
    start_study_session as core_start_study_session,
    submit_study_answer as core_submit_study_answer,
};
use word_storage_core::{
    models::{
        PlanSummary, TodayCompletionSeed, TodayHomeState, TodayHomeStateSeed, TodayTargetSeed,
        WordbookSummary,
    },
    persistence,
};

use crate::ai_agent::{
    base_url_without_suffix, extract_json_payload, strip_code_fences, AiAgent, AiProviderConfig,
    AiProviderProfile, ImageInput, ANTHROPIC_MESSAGES_PATH, OPENAI_RESPONSES_PATH,
};
use crate::paths::MobilePaths;
use crate::runtime::MobileRuntime;

// Global runtime state
static MOBILE_RUNTIME: Lazy<Mutex<Option<MobileRuntime>>> = Lazy::new(|| Mutex::new(None));
static WORD_HINT_RECOMMENDATION_LIBRARY: Lazy<BTreeMap<String, Vec<serde_json::Value>>> =
    Lazy::new(load_word_hint_recommendation_library);
const WORD_HINT_RECOMMENDATIONS_JSON: &str =
    include_str!("../resources/word_hint_recommendations.json");

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

/// Get a lightweight status payload for the currently loaded Rust bridge.
pub fn get_bridge_status() -> Result<String, String> {
    let runtime_guard = MOBILE_RUNTIME
        .lock()
        .map_err(|_| "Failed to lock runtime".to_string())?;

    if let Some(runtime) = runtime_guard.as_ref() {
        let db_path = runtime.paths().database_path();
        let payload = serde_json::json!({
            "bridge": "rust",
            "runtimeInitialized": true,
            "databasePath": db_path.display().to_string(),
        });
        serde_json::to_string(&payload).map_err(|e| format!("JSON serialization failed: {}", e))
    } else {
        let payload = serde_json::json!({
            "bridge": "rust",
            "runtimeInitialized": false,
        });
        serde_json::to_string(&payload).map_err(|e| format!("JSON serialization failed: {}", e))
    }
}

/// Get the global runtime instance.
fn get_runtime() -> Result<MutexGuard<'static, Option<MobileRuntime>>, String> {
    MOBILE_RUNTIME
        .lock()
        .map_err(|_| "Failed to lock runtime".to_string())
}

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

    let state = core_bootstrap_with_connection(runtime, &conn)
        .map_err(|e| format!("Bootstrap failed: {}", e))?;

    serde_json::to_string(&state).map_err(|e| format!("JSON serialization failed: {}", e))
}

pub fn mark_onboarding_completed() -> Result<(), String> {
    with_runtime_conn(|conn| {
        persistence::mark_onboarding_completed(conn)
            .map_err(|e| format!("Failed to mark onboarding completed: {}", e))
    })
}

// ============================================================================
// Today Home API
// ============================================================================

/// Get the today home state.
///
/// Returns a JSON string containing TodayHomeState.
pub fn get_today_home_state() -> Result<String, String> {
    with_runtime(|runtime, conn| {
        ensure_seed_vocabulary_imported(conn, &runtime.paths().bundled_resource_path(""))?;
        let state = build_authoritative_today_home_state(conn)?;
        serde_json::to_string(&state).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn get_reports_overview() -> Result<String, String> {
    with_runtime_conn(|conn| {
        repair_legacy_question_type_values(conn)?;
        let history = load_reports_history(conn)?;
        if history.is_empty() {
            if let Some(restored_overview) = load_restored_cloud_report_overview(conn)? {
                return serde_json::to_string(&restored_overview)
                    .map_err(|e| format!("JSON serialization failed: {}", e));
            }
        }
        let today_date = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let learned_count = load_distinct_learned_count(conn)?;
        let payload = reports_domain::build_reports_overview(&today_date, &history, learned_count);
        let outbox_payload = serde_json::json!({
            "type": "report_snapshot",
            "snapshotDate": today_date,
            "overview": payload,
        });
        enqueue_sync_snapshot(
            conn,
            "report_snapshot",
            &outbox_payload,
            &format!(
                "report_snapshot:{}",
                chrono::Local::now().date_naive().format("%Y-%m-%d")
            ),
        );
        serde_json::to_string(&payload).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn get_wrong_words(filter: String) -> Result<String, String> {
    with_runtime(|runtime, conn| {
        repair_seed_meaning_noise(conn)?;
        let mut entries = load_wrong_word_entries(conn)?;
        enrich_root_affix_entries_from_assets(
            &mut entries,
            &runtime.paths().bundled_resource_path(""),
        )?;
        let payload = wrong_words_domain::build_wrong_words(&mut entries, &filter);
        clean_json_string(&payload)
    })
}

pub fn get_wrong_word_detail(entry_id: i64) -> Result<String, String> {
    with_runtime(|runtime, conn| {
        repair_seed_meaning_noise(conn)?;
        let payload = load_wrong_word_detail_payload_with_bundle(
            conn,
            entry_id,
            Some(&runtime.paths().bundled_resource_path("")),
        )?;
        clean_json_string(&payload)
    })
}

pub fn save_word_hint(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    let entry_id = request
        .get("entryId")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "Missing entryId".to_string())?;
    if entry_id <= 0 {
        return Err("Hints require a mapped vocabulary entry".to_string());
    }
    let hint_text = request
        .get("hintText")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let source = request
        .get("source")
        .and_then(|value| value.as_str())
        .unwrap_or("user");

    with_runtime_conn(|conn| {
        let hint = persistence::word_hint_repo::save_hint(conn, entry_id, hint_text, source)
            .map_err(|e| e.to_string())?;
        enqueue_wrong_word_entries_snapshot(conn);
        clean_json_string(&word_hint_payload(entry_id, hint.as_ref()))
    })
}

pub fn get_word_hint_suggestions(entry_id: i64) -> Result<String, String> {
    with_runtime_conn(|conn| clean_json_string(&hint_suggestions_for_entry_v2(conn, entry_id)?))
}

pub fn get_today_ai_passage_context() -> Result<String, String> {
    with_runtime(|runtime, conn| {
        ensure_seed_vocabulary_imported(conn, &runtime.paths().bundled_resource_path(""))?;
        let today_state = build_authoritative_today_home_state(conn)?;
        let wrong_words = load_wrong_word_inputs(conn, AI_PASSAGE_MAX_WRONG_WORDS)?;

        let tasks_complete = today_state
            .today_snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot.new_words_completed >= snapshot.new_words_target
                    && snapshot.review_words_completed >= snapshot.review_words_target
                    && snapshot.mixed_test_completed >= snapshot.mixed_test_target
                    && snapshot.wrong_word_test_completed >= snapshot.wrong_word_test_target
                    && snapshot.root_affix_completed.unwrap_or(0)
                        >= snapshot.root_affix_target.unwrap_or(0)
            })
            .unwrap_or(false);

        let payload = serde_json::json!({
            "date": today_state.today_date,
            "tasksComplete": tasks_complete,
            "wrongWords": wrong_words
        });
        clean_json_string(&payload)
    })
}

fn build_authoritative_today_home_state(
    conn: &word_storage_core::Connection,
) -> Result<TodayHomeState, String> {
    ensure_planning_state(conn)?;

    let today_date = today_date_string();
    let mut plan_value = get_json_setting(conn, "today_plan_json", &default_plan_json())?;
    let mut wordbook_value = get_json_setting(
        conn,
        "today_wordbooks_json",
        &default_wordbook_selection_json(),
    )?;

    let targets = match load_today_target_seed(conn, &today_date)? {
        Some(targets) => targets,
        None => {
            let saved_plan = get_json_setting(conn, "saved_plan_json", &default_plan_json())?;
            let saved_wordbooks = get_json_setting(
                conn,
                "saved_wordbooks_json",
                &default_wordbook_selection_json(),
            )?;

            // If the user already has a saved plan but today's snapshot is missing,
            // treat that as an implicit "apply to today" so the dashboard can render.
            set_json_setting(conn, "today_plan_json", &saved_plan)?;
            set_json_setting(conn, "today_wordbooks_json", &saved_wordbooks)?;
            set_json_setting(conn, "today_review_wordbooks_json", &saved_wordbooks)?;

            plan_value = saved_plan;
            wordbook_value = saved_wordbooks;
            persist_today_target_seed(conn, &today_date, &plan_value)?
        }
    };
    let targets = align_today_targets_to_available_pools(conn, targets, &plan_value)?;

    let active_plan = Some(plan_summary_from_value(&plan_value));
    let wordbooks = assemble_wordbooks(&wordbook_value)
        .into_iter()
        .map(wordbook_summary_from_value)
        .collect::<Result<Vec<_>, _>>()?;

    let completions = load_today_completion_seed(conn, &today_date)?;

    Ok(core_build_today_home_state(TodayHomeStateSeed {
        today_date,
        active_plan,
        wordbooks,
        completions,
        targets,
    }))
}

fn plan_summary_from_value(value: &serde_json::Value) -> PlanSummary {
    PlanSummary {
        id: value.get("id").and_then(|v| v.as_i64()).unwrap_or(1),
        name: value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Starter Plan")
            .to_string(),
        new_words_per_day: value
            .get("newWordsPerDay")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        review_words_per_day: value
            .get("reviewWordsPerDay")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        mixed_test_per_day: value
            .get("mixedTestPerDay")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        wrong_word_test_per_day: value
            .get("wrongWordTestPerDay")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        root_affix_per_day: value.get("rootAffixPerDay").and_then(|v| v.as_i64()),
        growth_interval_days: value
            .get("growthIntervalDays")
            .and_then(|v| v.as_i64())
            .unwrap_or(7),
        growth_increment: value
            .get("growthIncrement")
            .and_then(|v| v.as_i64())
            .unwrap_or(5),
    }
}

fn wordbook_summary_from_value(value: serde_json::Value) -> Result<WordbookSummary, String> {
    Ok(WordbookSummary {
        id: value.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
        code: value
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        name: value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        category: value
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        total_entries: value
            .get("totalEntries")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        is_active: value
            .get("isActive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

fn load_today_completion_seed(
    conn: &word_storage_core::Connection,
    today_date: &str,
) -> Result<TodayCompletionSeed, String> {
    let active = load_active_session_completion_seed(conn, today_date)?;

    let mut stmt = conn
        .prepare(
            "SELECT s.session_id, s.mode, r.answered_at
             FROM study_results r
             INNER JOIN study_sessions s ON s.session_id = r.session_id
             WHERE r.answered_at IS NOT NULL",
        )
        .map_err(|e| format!("Failed to prepare today completion query: {e}"))?;

    let mut new_words_completed = 0u32;
    let mut review_words_completed = 0u32;
    let mut mixed_test_completed = 0u32;
    let mut wrong_word_test_completed = 0u32;
    let mut root_affix_completed = 0u32;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("Failed to query today completions: {e}"))?;

    for row in rows {
        let (session_id, mode, answered_at) =
            row.map_err(|e| format!("Failed to decode today completion row: {e}"))?;
        if active.session_ids.contains(&session_id) {
            continue;
        }
        if local_date_from_rfc3339(&answered_at).as_deref() != Some(today_date) {
            continue;
        }
        match normalize_stored_session_mode(&mode).as_str() {
            "newWord" => new_words_completed = new_words_completed.saturating_add(1),
            "review" => review_words_completed = review_words_completed.saturating_add(1),
            "mixedTest" => mixed_test_completed = mixed_test_completed.saturating_add(1),
            "wrongWordReinforcement" => {
                wrong_word_test_completed = wrong_word_test_completed.saturating_add(1)
            }
            "rootAffix" => root_affix_completed = root_affix_completed.saturating_add(1),
            _ => {}
        }
    }

    new_words_completed = new_words_completed.saturating_add(active.seed.new_words_completed);
    review_words_completed =
        review_words_completed.saturating_add(active.seed.review_words_completed);
    mixed_test_completed = mixed_test_completed.saturating_add(active.seed.mixed_test_completed);
    wrong_word_test_completed =
        wrong_word_test_completed.saturating_add(active.seed.wrong_word_test_completed);
    root_affix_completed =
        root_affix_completed.saturating_add(active.seed.root_affix_completed.unwrap_or(0));

    Ok(TodayCompletionSeed {
        new_words_completed,
        review_words_completed,
        mixed_test_completed,
        wrong_word_test_completed,
        root_affix_completed: Some(root_affix_completed),
    })
}

fn normalize_stored_session_mode(mode: &str) -> String {
    serde_json::from_str::<String>(mode).unwrap_or_else(|_| mode.to_string())
}

fn local_date_from_rfc3339(value: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date_time| {
            date_time
                .with_timezone(&chrono::Local)
                .date_naive()
                .format("%Y-%m-%d")
                .to_string()
        })
}

struct ActiveSessionCompletionSeed {
    seed: TodayCompletionSeed,
    session_ids: BTreeSet<String>,
}

fn load_active_session_completion_seed(
    conn: &word_storage_core::Connection,
    today_date: &str,
) -> Result<ActiveSessionCompletionSeed, String> {
    let mut seed = TodayCompletionSeed {
        new_words_completed: 0,
        review_words_completed: 0,
        mixed_test_completed: 0,
        wrong_word_test_completed: 0,
        root_affix_completed: Some(0),
    };
    let mut session_ids = BTreeSet::new();

    let active_modes = [
        ("active_study_session_newWord", "newWord"),
        ("active_study_session_review", "review"),
        ("active_study_session_mixedTest", "mixedTest"),
        (
            "active_study_session_wrongWordReinforcement",
            "wrongWordReinforcement",
        ),
        ("active_study_session_rootAffix", "rootAffix"),
    ];

    for (key, mode) in active_modes {
        let raw = conn
            .query_row(
                "SELECT value_json FROM app_settings WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("Failed to load active session completion: {e}"))?;
        let Some(raw) = raw else {
            continue;
        };
        let snapshot: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("Invalid active session completion snapshot: {e}"))?;
        let started_at = snapshot
            .get("session")
            .and_then(|session| session.get("startedAt"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if local_date_from_rfc3339(started_at).as_deref() != Some(today_date) {
            continue;
        }
        if let Some(session_id) = snapshot
            .get("session")
            .and_then(|session| session.get("sessionId"))
            .and_then(|value| value.as_str())
        {
            session_ids.insert(session_id.to_string());
        }
        let answered = snapshot
            .get("results")
            .and_then(|value| value.as_array())
            .map(|items| items.len() as u32)
            .or_else(|| {
                snapshot
                    .get("currentIndex")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as u32)
            })
            .unwrap_or(0);

        match mode {
            "newWord" => seed.new_words_completed = answered,
            "review" => seed.review_words_completed = answered,
            "mixedTest" => seed.mixed_test_completed = answered,
            "wrongWordReinforcement" => seed.wrong_word_test_completed = answered,
            "rootAffix" => seed.root_affix_completed = Some(answered),
            _ => {}
        }
    }

    Ok(ActiveSessionCompletionSeed { seed, session_ids })
}

pub fn get_resume_session_hint() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let mut payload = load_resume_session_hint(conn)?;
        clean_seed_meaning_noise_in_json(&mut payload);
        serde_json::to_string(&payload).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

fn load_resume_session_hint(
    conn: &word_storage_core::Connection,
) -> Result<serde_json::Value, String> {
    use word_storage_core::models::SessionMode;

    let ordered_modes = [
        SessionMode::NewWord,
        SessionMode::Review,
        SessionMode::MixedTest,
        SessionMode::WrongWordReinforcement,
        SessionMode::RootAffix,
    ];

    for mode in ordered_modes {
        let raw = persistence::study_repo::load_active_session_snapshot(conn, &mode)
            .map_err(|e| format!("Failed to load active session snapshot: {e}"))?;
        let Some(raw) = raw else {
            continue;
        };
        let snapshot: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|e| format!("Invalid active session snapshot: {e}"))?;
        let current = snapshot
            .get("currentIndex")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let questions = snapshot
            .get("questions")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        let total = questions.len() as u64;
        let current_question = questions.get(current as usize);
        let word = current_question
            .and_then(|value| value.get("word"))
            .and_then(|value| value.as_str())
            .unwrap_or("");

        return Ok(serde_json::json!({
            "hasResume": true,
            "mode": serde_json::to_string(&mode).unwrap_or_else(|_| "\"newWord\"".to_string()).trim_matches('"').to_string(),
            "current": current + 1,
            "total": total,
            "word": word,
        }));
    }

    Ok(serde_json::json!({
        "hasResume": false,
        "mode": null,
        "current": null,
        "total": null,
        "word": null
    }))
}

pub fn build_today_home_state(request_json: String) -> Result<String, String> {
    let seed: TodayHomeStateSeed =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let state = core_build_today_home_state(seed);
    serde_json::to_string(&state).map_err(|e| format!("JSON serialization failed: {}", e))
}

fn load_reports_history(conn: &rusqlite::Connection) -> Result<Vec<serde_json::Value>, String> {
    use std::collections::BTreeMap;

    let mut results_stmt = conn
        .prepare(
            "SELECT s.session_id, s.mode,
                    r.question_id, r.entry_id, r.question_type, r.user_response,
                    r.normalized_response, r.correct_answer, r.outcome,
                    r.response_time_ms, r.answered_at
             FROM study_results r
             JOIN study_sessions s ON s.session_id = r.session_id
             ORDER BY r.answered_at ASC, r.id ASC",
        )
        .map_err(|e| format!("Failed to prepare reports result query: {e}"))?;

    let rows = results_stmt
        .query_map([], |row| {
            let session_id: String = row.get(0)?;
            let mode: String = row.get(1)?;
            let question_type: String = row.get(4)?;
            let outcome: String = row.get(8)?;
            let entry_id: i64 = row.get(3)?;
            let answered_at: String = row.get(10)?;
            let answer_date = local_date_from_rfc3339(&answered_at)
                .unwrap_or_else(|| answered_at.get(0..10).unwrap_or(&answered_at).to_string());
            Ok((
                answer_date,
                normalize_persisted_enum_text(&mode),
                session_id,
                serde_json::json!({
                    "questionId": row.get::<_, String>(2)?,
                    "entrySourceId": format!("entry_{entry_id}"),
                    "questionType": normalize_persisted_enum_text(&question_type),
                    "userResponse": row.get::<_, String>(5)?,
                    "normalizedResponse": row.get::<_, Option<String>>(6)?,
                    "correctAnswer": row.get::<_, String>(7)?,
                    "outcome": normalize_persisted_enum_text(&outcome),
                    "responseTimeMs": row.get::<_, i64>(9)?,
                    "answeredAt": answered_at,
                }),
            ))
        })
        .map_err(|e| format!("Failed to query reports results: {e}"))?;

    let mut grouped_results = BTreeMap::<(String, String), Vec<serde_json::Value>>::new();
    let mut group_session_ids = BTreeMap::<(String, String), BTreeMap<String, usize>>::new();
    for row in rows {
        let (date, mode, session_id, result) =
            row.map_err(|e| format!("Failed to decode reports result row: {e}"))?;
        if date.is_empty() {
            continue;
        }
        let key = (date, mode);
        group_session_ids
            .entry(key.clone())
            .or_default()
            .entry(session_id)
            .and_modify(|count| *count += 1)
            .or_insert(1);
        grouped_results.entry(key).or_default().push(result);
    }

    let mut history = Vec::new();
    for ((date, mode), session_results) in grouped_results {
        if session_results.is_empty() {
            continue;
        }
        let session_label = group_session_ids
            .get(&(date.clone(), mode.clone()))
            .and_then(|sessions| sessions.keys().next().cloned())
            .unwrap_or_else(|| format!("{mode}_{date}"));
        let summary_value = recompute_summary_json(&session_label, &session_results, &date)?;
        history.push(serde_json::json!({
            "date": date,
            "mode": mode,
            "summary": summary_value
        }));
    }

    // keep deterministic date ordering for report service consumers
    let mut grouped = BTreeMap::<String, Vec<serde_json::Value>>::new();
    for item in history {
        let key = item
            .get("date")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        grouped.entry(key).or_default().push(item);
    }
    Ok(grouped.into_values().flatten().collect())
}

fn load_restored_cloud_report_overview(
    conn: &rusqlite::Connection,
) -> Result<Option<serde_json::Value>, String> {
    let snapshots = get_json_setting(
        conn,
        "cloud_report_snapshots_json",
        &serde_json::Value::Array(Vec::new()),
    )?;
    let Some(items) = snapshots.as_array() else {
        return Ok(None);
    };
    let overview = items.iter().rev().find_map(|item| {
        item.get("payloadJson")
            .or_else(|| item.get("payload_json"))
            .cloned()
    });
    Ok(overview.map(normalize_restored_cloud_report_overview))
}

fn normalize_restored_cloud_report_overview(mut overview: serde_json::Value) -> serde_json::Value {
    let Some(obj) = overview.as_object_mut() else {
        return serde_json::json!({
            "totalStudyDays": 0,
            "totalWordsLearned": 0,
            "totalQuestionsAnswered": 0,
            "overallAccuracy": 0.0,
            "streakInfo": {},
            "modeBreakdown": [],
            "last7Days": [],
            "dailySeries": [],
            "modeSeries": {}
        });
    };

    let last7_days = obj
        .get("last7Days")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if !obj
        .get("dailySeries")
        .and_then(|value| value.as_array())
        .is_some_and(|items| !items.is_empty())
        && !last7_days.is_empty()
    {
        obj.insert(
            "dailySeries".to_string(),
            serde_json::Value::Array(last7_days.clone()),
        );
    }

    let series = obj
        .get("dailySeries")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let total_questions = series
        .iter()
        .map(|item| {
            item.get("totalQuestions")
                .or_else(|| item.get("questionsAnswered"))
                .and_then(json_i64_value)
                .unwrap_or(0)
        })
        .sum::<i64>()
        .max(0);
    let correct_count = series
        .iter()
        .map(|item| {
            item.get("correctCount")
                .and_then(json_i64_value)
                .unwrap_or(0)
        })
        .sum::<i64>()
        .max(0);
    let study_days = series
        .iter()
        .filter(|item| {
            item.get("totalQuestions")
                .or_else(|| item.get("questionsAnswered"))
                .and_then(json_i64_value)
                .unwrap_or(0)
                > 0
        })
        .count() as i64;

    obj.entry("totalStudyDays".to_string())
        .or_insert_with(|| serde_json::Value::from(study_days));
    obj.entry("totalWordsLearned".to_string())
        .or_insert_with(|| serde_json::Value::from(0));
    obj.entry("totalQuestionsAnswered".to_string())
        .or_insert_with(|| serde_json::Value::from(total_questions));
    obj.entry("overallAccuracy".to_string()).or_insert_with(|| {
        serde_json::Value::from(if total_questions > 0 {
            (correct_count as f64 * 100.0) / total_questions as f64
        } else {
            0.0
        })
    });
    obj.entry("streakInfo".to_string())
        .or_insert_with(|| serde_json::json!({}));
    obj.entry("modeBreakdown".to_string())
        .or_insert_with(|| serde_json::json!([]));
    obj.entry("last7Days".to_string())
        .or_insert_with(|| serde_json::Value::Array(series.clone()));
    obj.entry("dailySeries".to_string())
        .or_insert_with(|| serde_json::Value::Array(series));
    obj.entry("modeSeries".to_string())
        .or_insert_with(|| serde_json::json!({}));
    overview
}

fn normalize_persisted_enum_text(value: &str) -> String {
    serde_json::from_str::<String>(value).unwrap_or_else(|_| value.to_string())
}

fn repair_legacy_question_type_values(conn: &rusqlite::Connection) -> Result<(), String> {
    conn.execute(
        "UPDATE study_results
         SET question_type = 'enToCnChoice'
         WHERE question_type IN ('meaning', 'choice', 'unknown', '\"meaning\"', '\"choice\"', '\"unknown\"')",
        [],
    )
    .map_err(|e| format!("Failed to repair legacy meaning question types: {e}"))?;
    conn.execute(
        "UPDATE study_results
         SET question_type = 'enToCnInput'
         WHERE question_type IN ('spelling', 'input', '\"spelling\"', '\"input\"')",
        [],
    )
    .map_err(|e| format!("Failed to repair legacy spelling question types: {e}"))?;
    Ok(())
}

fn load_distinct_learned_count(conn: &rusqlite::Connection) -> Result<u64, String> {
    conn.query_row(
        "SELECT COUNT(DISTINCT entry_id)
         FROM study_results
         WHERE outcome IN ('correct', 'fuzzyCorrect', '\"correct\"', '\"fuzzyCorrect\"')",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count.max(0) as u64)
    .map_err(|e| format!("Failed to compute learned count: {e}"))
}

fn load_wrong_word_entries(conn: &rusqlite::Connection) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
                e.id,
                e.source_entry_key,
                e.word,
                e.phonetic_us,
                e.phonetic_uk,
                e.part_of_speech,
                SUM(CASE WHEN sr.outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"') THEN 1 ELSE 0 END) as error_count,
                MAX(CASE WHEN sr.outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"') THEN sr.answered_at ELSE NULL END) as last_wrong_at,
                SUM(CASE WHEN sr.question_type IN ('glossToRootInput', 'rootToGlossInput', '\"glossToRootInput\"', '\"rootToGlossInput\"') THEN 1 ELSE 0 END) as root_affix_attempts
             FROM study_results sr
             JOIN entries e ON e.id = sr.entry_id
             WHERE sr.outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"')
             GROUP BY e.id, e.source_entry_key, e.word, e.phonetic_us, e.phonetic_uk, e.part_of_speech
             ORDER BY last_wrong_at DESC",
        )
        .map_err(|e| format!("Failed to prepare wrong-word list query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let entry_id: i64 = row.get(0)?;
            let part_of_speech = row.get::<_, Option<String>>(5)?.unwrap_or_default();
            let root_affix_attempts = row.get::<_, i64>(8)?;
            let entry_kind =
                if root_affix_attempts > 0 || is_root_affix_part_of_speech(&part_of_speech) {
                    "rootAffix"
                } else {
                    "word"
                };
            Ok((
                entry_id,
                serde_json::json!({
                    "entryId": entry_id,
                    "sourceEntryKey": row.get::<_, String>(1)?,
                    "word": row.get::<_, String>(2)?,
                    "phoneticUs": row.get::<_, Option<String>>(3)?,
                    "phoneticUk": row.get::<_, Option<String>>(4)?,
                    "partOfSpeech": part_of_speech,
                    "errorCount": row.get::<_, i64>(6)?,
                    "lastWrongAt": row.get::<_, Option<String>>(7)?,
                    "entryKind": entry_kind,
                    "isActive": true
                }),
            ))
        })
        .map_err(|e| format!("Failed to query wrong-word list: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        let (entry_id, mut value) =
            row.map_err(|e| format!("Failed to decode wrong-word list row: {e}"))?;
        value["meanings"] = serde_json::Value::Array(load_entry_meaning_strings(conn, entry_id)?);
        attach_word_hint_fields(conn, entry_id, &mut value)?;
        result.push(value);
    }
    result.extend(load_imported_wrong_word_entries(conn)?);
    Ok(result)
}

fn load_imported_wrong_word_entries(
    conn: &rusqlite::Connection,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT
                MIN(id) as import_id,
                entry_id,
                word,
                MAX(COALESCE(meaning, '')) as meaning,
                SUM(occurrence_count) as occurrence_count,
                MAX(imported_at) as imported_at,
                MAX(is_high_frequency) as is_high_frequency,
                MAX(confidence) as confidence
             FROM imported_wrong_words
             GROUP BY COALESCE(entry_id, -id), LOWER(word)
             ORDER BY imported_at DESC",
        )
        .map_err(|e| format!("Failed to prepare imported wrong-word query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let import_id: i64 = row.get(0)?;
            let entry_id: Option<i64> = row.get(1)?;
            let meaning: String = row.get(3)?;
            let occurrence_count: i64 = row.get(4)?;
            Ok(serde_json::json!({
                "entryId": entry_id.unwrap_or(-import_id),
                "word": row.get::<_, String>(2)?,
                "phoneticUs": null,
                "phoneticUk": null,
                "meanings": if meaning.trim().is_empty() {
                    serde_json::json!([])
                } else {
                    serde_json::json!([meaning])
                },
                "errorCount": occurrence_count.max(1),
                "lastWrongAt": row.get::<_, String>(5)?,
                "entryKind": "word",
                "isActive": true,
                "isImported": true,
                "importConfidence": row.get::<_, f64>(7)?,
                "isHighFrequency": row.get::<_, i64>(6)? != 0
            }))
        })
        .map_err(|e| format!("Failed to query imported wrong words: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode imported wrong-word row: {e}"))
}

fn load_wrong_word_detail_payload_with_bundle(
    conn: &rusqlite::Connection,
    entry_id: i64,
    bundle_dir: Option<&Path>,
) -> Result<serde_json::Value, String> {
    if entry_id < 0 {
        return load_imported_wrong_word_detail_payload(conn, -entry_id);
    }

    let (source_entry_key, word, lemma, phonetic_us, phonetic_uk, part_of_speech) = conn
        .query_row(
            "SELECT source_entry_key, word, lemma, phonetic_us, phonetic_uk, part_of_speech
             FROM entries
             WHERE id = ?1",
            [entry_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            },
        )
        .map_err(|e| format!("Failed to load wrong-word entry detail: {e}"))?;

    let part_of_speech = part_of_speech.unwrap_or_default();
    let mut meanings = load_entry_meaning_details(conn, entry_id)?;
    let mut examples = load_entry_examples(conn, entry_id)?;
    if is_root_affix_part_of_speech(&part_of_speech) {
        if let Some(bundle_dir) = bundle_dir {
            if let Some(card) = load_root_affix_card_by_id(bundle_dir, &source_entry_key)? {
                if meanings.is_empty() {
                    meanings.push(root_affix_card_meaning_json(&card));
                }
                if examples.is_empty() {
                    examples = root_affix_card_examples_json(&card);
                }
            }
        }
    }

    let mut error_history_stmt = conn
        .prepare(
            "SELECT answered_at, question_type, outcome
             FROM study_results
             WHERE entry_id = ?1 AND outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"')
             ORDER BY answered_at DESC
             LIMIT 20",
        )
        .map_err(|e| format!("Failed to prepare wrong-word history query: {e}"))?;

    let error_history = error_history_stmt
        .query_map([entry_id], |row| {
            Ok(serde_json::json!({
                "date": row.get::<_, String>(0)?,
                "context": format!(
                    "{} / {}",
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?
                )
            }))
        })
        .map_err(|e| format!("Failed to query wrong-word history: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode wrong-word history row: {e}"))?;

    let mut breakdown_stmt = conn
        .prepare(
            "SELECT
                question_type,
                COUNT(*) as attempts,
                SUM(CASE WHEN outcome IN ('incorrect', '\"incorrect\"') THEN 1 ELSE 0 END) as incorrect_count,
                SUM(CASE WHEN outcome IN ('skipped', '\"skipped\"') THEN 1 ELSE 0 END) as skipped_count,
                SUM(CASE WHEN outcome IN ('fuzzyCorrect', '\"fuzzyCorrect\"') THEN 1 ELSE 0 END) as fuzzy_count,
                SUM(CASE WHEN outcome IN ('correct', '\"correct\"') THEN 1 ELSE 0 END) as correct_count
             FROM study_results
             WHERE entry_id = ?1
             GROUP BY question_type
             ORDER BY question_type ASC",
        )
        .map_err(|e| format!("Failed to prepare wrong-word risk breakdown query: {e}"))?;

    let risk_breakdown = breakdown_stmt
        .query_map([entry_id], |row| {
            Ok(serde_json::json!({
                "questionType": row.get::<_, String>(0)?,
                "attempts": row.get::<_, i64>(1)?,
                "incorrect": row.get::<_, i64>(2)?,
                "skipped": row.get::<_, i64>(3)?,
                "fuzzyCorrect": row.get::<_, i64>(4)?,
                "correct": row.get::<_, i64>(5)?,
            }))
        })
        .map_err(|e| format!("Failed to query wrong-word breakdown: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode wrong-word breakdown row: {e}"))?;

    let mut payload = serde_json::json!({
        "entryId": entry_id,
        "word": word,
        "lemma": lemma,
        "phoneticUs": phonetic_us,
        "phoneticUk": phonetic_uk,
        "partOfSpeech": part_of_speech,
        "meanings": meanings,
        "examples": examples,
        "errorHistory": error_history,
        "riskBreakdown": risk_breakdown,
        "errorCount": persistence::word_hint_repo::counted_error_count(conn, entry_id)
            .map_err(|e| e.to_string())?,
        "relatedWords": Vec::<String>::new()
    });
    attach_word_hint_fields(conn, entry_id, &mut payload)?;
    Ok(payload)
}

fn load_imported_wrong_word_detail_payload(
    conn: &rusqlite::Connection,
    import_id: i64,
) -> Result<serde_json::Value, String> {
    let (
        word,
        meaning,
        occurrence_count,
        confidence,
        evidence,
        source_type,
        source_name,
        imported_at,
        is_high_frequency,
    ) = conn
        .query_row(
            "SELECT word, COALESCE(meaning, ''), occurrence_count, confidence, evidence,
                    source_type, source_name, imported_at, is_high_frequency
             FROM imported_wrong_words
             WHERE id = ?1",
            [import_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, f64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, i64>(8)? != 0,
                ))
            },
        )
        .map_err(|e| format!("Failed to load imported wrong-word detail: {e}"))?;

    let meanings = if meaning.trim().is_empty() {
        Vec::new()
    } else {
        vec![serde_json::json!({
            "pos": "",
            "meaningCn": meaning,
            "meaningEn": null
        })]
    };

    Ok(serde_json::json!({
        "entryId": -import_id,
        "word": word,
        "lemma": word,
        "phoneticUs": null,
        "phoneticUk": null,
        "partOfSpeech": "",
        "meanings": meanings,
        "examples": [],
        "errorHistory": [{
            "date": imported_at,
            "context": format!("imported wrong word / {source_type}")
        }],
        "riskBreakdown": [{
            "questionType": "importedWrongWord",
            "attempts": occurrence_count,
            "incorrect": occurrence_count,
            "skipped": 0,
            "fuzzyCorrect": 0,
            "correct": 0,
            "confidence": confidence,
            "sourceName": source_name,
            "evidence": evidence,
            "isHighFrequency": is_high_frequency
        }],
        "relatedWords": []
    }))
}

fn attach_word_hint_fields(
    conn: &rusqlite::Connection,
    entry_id: i64,
    value: &mut serde_json::Value,
) -> Result<(), String> {
    let hint = persistence::word_hint_repo::get_hint(conn, entry_id).map_err(|e| e.to_string())?;
    value["userHint"] = hint
        .as_ref()
        .map(|hint| serde_json::Value::String(hint.hint_text.clone()))
        .unwrap_or(serde_json::Value::Null);
    value["hintSource"] = hint
        .as_ref()
        .map(|hint| serde_json::Value::String(hint.source.clone()))
        .unwrap_or(serde_json::Value::Null);
    value["hintUpdatedAt"] = hint
        .as_ref()
        .map(|hint| serde_json::Value::String(hint.updated_at.clone()))
        .unwrap_or(serde_json::Value::Null);
    value["hasHint"] = serde_json::json!(hint.is_some());
    value["hintSuggestions"] = hint_suggestions_for_entry_v2(conn, entry_id)?;
    Ok(())
}

fn enrich_study_response_hints(
    conn: &rusqlite::Connection,
    payload: &mut serde_json::Value,
) -> Result<(), String> {
    if let Some(question) = payload
        .get_mut("currentQuestion")
        .filter(|value| value.is_object())
    {
        enrich_study_question_hints(conn, question)?;
    }
    Ok(())
}

fn enrich_submit_response_hints(
    conn: &rusqlite::Connection,
    payload: &mut serde_json::Value,
) -> Result<(), String> {
    if let Some(question) = payload
        .get_mut("currentQuestion")
        .filter(|value| value.is_object())
    {
        enrich_study_question_hints(conn, question)?;
    }
    let result = payload
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let current_question = payload
        .get("currentQuestion")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    payload["hintPrompt"] = build_hint_prompt_payload(conn, &result, &current_question)?;
    Ok(())
}

fn enrich_study_question_hints(
    conn: &rusqlite::Connection,
    question: &mut serde_json::Value,
) -> Result<(), String> {
    let Some(entry_id) = question_entry_id(question) else {
        question["userHint"] = serde_json::Value::Null;
        question["hasHint"] = serde_json::json!(false);
        question["hintSuggestions"] = serde_json::json!([]);
        return Ok(());
    };
    attach_word_hint_fields(conn, entry_id, question)
}

fn build_hint_prompt_payload(
    conn: &rusqlite::Connection,
    result: &serde_json::Value,
    current_question: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let outcome = result
        .get("outcome")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    if !matches!(
        outcome,
        "incorrect" | "skipped" | "\"incorrect\"" | "\"skipped\""
    ) {
        return Ok(serde_json::Value::Null);
    }
    let Some(entry_id) = result_entry_id(result).or_else(|| question_entry_id(current_question))
    else {
        return Ok(serde_json::Value::Null);
    };
    if persistence::word_hint_repo::get_hint(conn, entry_id)
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Ok(serde_json::Value::Null);
    }
    let counted_errors = persistence::word_hint_repo::counted_error_count(conn, entry_id)
        .map_err(|e| e.to_string())?;
    if counted_errors < 5 {
        return Ok(serde_json::Value::Null);
    }
    let word = load_entry_word(conn, entry_id).unwrap_or_else(|_| {
        result
            .get("entrySourceId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string()
    });
    Ok(serde_json::json!({
        "entryId": entry_id,
        "word": word,
        "errorCount": counted_errors,
        "triggerOutcome": normalize_stored_enum_text(outcome),
        "suggestions": hint_suggestions_for_entry_v2(conn, entry_id)?
    }))
}

fn word_hint_payload(
    entry_id: i64,
    hint: Option<&persistence::word_hint_repo::WordHint>,
) -> serde_json::Value {
    serde_json::json!({
        "entryId": entry_id,
        "userHint": hint.map(|hint| hint.hint_text.clone()),
        "hintSource": hint.map(|hint| hint.source.clone()),
        "hintUpdatedAt": hint.map(|hint| hint.updated_at.clone()),
        "hasHint": hint.is_some()
    })
}

fn hint_suggestions_for_entry_v2(
    conn: &rusqlite::Connection,
    entry_id: i64,
) -> Result<serde_json::Value, String> {
    if entry_id <= 0 {
        return Ok(serde_json::json!([]));
    }
    let (word, part_of_speech) = conn
        .query_row(
            "SELECT word, COALESCE(part_of_speech, '') FROM entries WHERE id = ?1",
            [entry_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| format!("Failed to load hint suggestion entry: {e}"))?
        .unwrap_or_else(|| ("".to_string(), "".to_string()));
    if word.trim().is_empty() {
        return Ok(serde_json::json!([]));
    }

    let wordbook_codes = load_entry_wordbook_codes(conn, entry_id)?;
    let mut suggestions = recommendation_library_for_word(&word, &wordbook_codes);
    if suggestions.is_empty() {
        let normalized_pos = part_of_speech.trim().trim_end_matches('.');
        suggestions.push(serde_json::json!({
            "id": format!("{}-meaning-visual", normalized_hint_word(&word)),
            "word": normalized_hint_word(&word),
            "style": "meaning",
            "label": "AI推荐",
            "wordbookCodes": wordbook_codes,
            "text": if normalized_pos.is_empty() {
                format!("看到 {word} 时，先想它在题目里的核心中文义，再排除相近干扰项。")
            } else {
                format!("{word} 是 {normalized_pos}，先用词性锁定答案范围，再回想核心中文义。")
            }
        }));
    }
    Ok(serde_json::Value::Array(suggestions))
}

fn recommendation_library_for_word(
    word: &str,
    wordbook_codes: &[String],
) -> Vec<serde_json::Value> {
    WORD_HINT_RECOMMENDATION_LIBRARY
        .get(&normalized_hint_word(word))
        .map(|items| {
            items
                .iter()
                .cloned()
                .map(|mut item| {
                    if !wordbook_codes.is_empty() {
                        item["wordbookCodes"] = serde_json::json!(wordbook_codes);
                    }
                    item
                })
                .collect()
        })
        .unwrap_or_default()
}

fn load_word_hint_recommendation_library() -> BTreeMap<String, Vec<serde_json::Value>> {
    let Ok(items) = serde_json::from_str::<Vec<serde_json::Value>>(WORD_HINT_RECOMMENDATIONS_JSON)
    else {
        return BTreeMap::new();
    };
    let mut by_word = BTreeMap::<String, Vec<serde_json::Value>>::new();
    for item in items {
        let word = item
            .get("word")
            .and_then(|value| value.as_str())
            .map(normalized_hint_word)
            .unwrap_or_default();
        if word.is_empty() {
            continue;
        }
        by_word.entry(word).or_default().push(item);
    }
    by_word
}

fn normalized_hint_word(word: &str) -> String {
    word.trim().to_ascii_lowercase()
}

fn load_entry_wordbook_codes(
    conn: &rusqlite::Connection,
    entry_id: i64,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT wb.code
             FROM wordbook_entries we
             JOIN wordbooks wb ON wb.id = we.wordbook_id
             WHERE we.entry_id = ?1
             ORDER BY wb.code ASC",
        )
        .map_err(|e| format!("Failed to prepare hint wordbook lookup: {e}"))?;
    let codes = stmt
        .query_map([entry_id], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Failed to query hint wordbook lookup: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode hint wordbook lookup: {e}"))?;
    Ok(codes)
}

fn question_entry_id(question: &serde_json::Value) -> Option<i64> {
    question
        .get("entrySourceId")
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|entry_id| *entry_id > 0)
}

fn result_entry_id(result: &serde_json::Value) -> Option<i64> {
    result
        .get("entrySourceId")
        .and_then(|value| value.as_str())
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|entry_id| *entry_id > 0)
}

fn load_entry_word(conn: &rusqlite::Connection, entry_id: i64) -> Result<String, String> {
    conn.query_row(
        "SELECT word FROM entries WHERE id = ?1",
        [entry_id],
        |row| row.get::<_, String>(0),
    )
    .map_err(|e| format!("Failed to load hint prompt word: {e}"))
}

fn normalize_stored_enum_text(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}

fn is_root_affix_part_of_speech(part_of_speech: &str) -> bool {
    let normalized = part_of_speech
        .trim()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    matches!(normalized.as_str(), "root" | "prefix" | "suffix" | "affix")
}

fn enrich_root_affix_entries_from_assets(
    entries: &mut [serde_json::Value],
    bundle_dir: &Path,
) -> Result<(), String> {
    let mut cards_by_id: Option<BTreeMap<String, RootAffixCard>> = None;
    for entry in entries.iter_mut() {
        if entry.get("entryKind").and_then(|value| value.as_str()) != Some("rootAffix") {
            continue;
        }
        let Some(source_entry_key) = entry.get("sourceEntryKey").and_then(|value| value.as_str())
        else {
            continue;
        };
        let cards = match cards_by_id.as_ref() {
            Some(cards) => cards,
            None => {
                cards_by_id = Some(root_affix_cards_by_id(bundle_dir)?);
                cards_by_id.as_ref().expect("cards initialized")
            }
        };
        let Some(card) = cards.get(source_entry_key) else {
            continue;
        };
        let meanings_empty = entry
            .get("meanings")
            .and_then(|value| value.as_array())
            .is_none_or(|values| values.is_empty());
        if meanings_empty {
            entry["meanings"] =
                serde_json::Value::Array(vec![serde_json::Value::String(card.meaning_cn.clone())]);
        }
    }
    Ok(())
}

fn load_root_affix_card_by_id(
    bundle_dir: &Path,
    source_entry_key: &str,
) -> Result<Option<RootAffixCard>, String> {
    Ok(root_affix_cards_by_id(bundle_dir)?.remove(source_entry_key))
}

fn root_affix_cards_by_id(bundle_dir: &Path) -> Result<BTreeMap<String, RootAffixCard>, String> {
    Ok(load_root_affix_cards(bundle_dir)?
        .into_iter()
        .map(|card| (card.id.clone(), card))
        .collect())
}

fn root_affix_card_meaning_json(card: &RootAffixCard) -> serde_json::Value {
    serde_json::json!({
        "pos": "root",
        "meaningCn": card.meaning_cn,
        "meaningEn": null
    })
}

fn root_affix_card_examples_json(card: &RootAffixCard) -> Vec<serde_json::Value> {
    card.example_pairs
        .iter()
        .map(|(word, gloss)| {
            serde_json::json!({
                "sentenceEn": word,
                "sentenceCn": gloss
            })
        })
        .collect()
}

fn load_wrong_word_inputs(
    conn: &rusqlite::Connection,
    limit: usize,
) -> Result<Vec<serde_json::Value>, String> {
    let mut entries = load_wrong_word_entries(conn)?;
    let ordered = wrong_words_domain::build_wrong_words(&mut entries, "all");
    let mut result = Vec::new();

    for entry in ordered
        .into_iter()
        .filter(|entry| {
            entry.get("entryKind").and_then(|value| value.as_str()) != Some("rootAffix")
        })
        .take(limit)
    {
        let entry_id = entry
            .get("entryId")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        let part_of_speech = entry
            .get("partOfSpeech")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                conn.query_row(
                    "SELECT part_of_speech FROM entries WHERE id = ?1",
                    [entry_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .unwrap_or(None)
            });
        let primary_gloss = entry
            .get("meanings")
            .and_then(|value| value.as_array())
            .and_then(|values| values.iter().find_map(|value| value.as_str()))
            .map(str::to_string)
            .unwrap_or_else(|| {
                load_entry_meaning_strings(conn, entry_id)
                    .unwrap_or_default()
                    .into_iter()
                    .find_map(|value| value.as_str().map(|s| s.to_string()))
                    .unwrap_or_default()
            });

        result.push(serde_json::json!({
            "entryId": entry_id,
            "word": entry.get("word").and_then(|value| value.as_str()).unwrap_or(""),
            "primaryGloss": primary_gloss,
            "partOfSpeech": part_of_speech
        }));
    }

    Ok(result)
}

fn load_entry_meaning_strings(
    conn: &rusqlite::Connection,
    entry_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT meaning_cn
             FROM entry_meanings
             WHERE entry_id = ?1
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| format!("Failed to prepare entry meanings query: {e}"))?;
    let values = stmt
        .query_map([entry_id], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Failed to query entry meanings: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map(|values| {
            values
                .into_iter()
                .map(|value| serde_json::Value::String(clean_seed_meaning_cn(&value)))
                .collect()
        })
        .map_err(|e| format!("Failed to decode entry meanings row: {e}"))?;
    Ok(values)
}

fn load_entry_meaning_details(
    conn: &rusqlite::Connection,
    entry_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT pos, meaning_cn, meaning_en
             FROM entry_meanings
             WHERE entry_id = ?1
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| format!("Failed to prepare entry meaning detail query: {e}"))?;
    let values = stmt
        .query_map([entry_id], |row| {
            let meaning_cn = row.get::<_, String>(1)?;
            Ok(serde_json::json!({
                "pos": row.get::<_, String>(0)?,
                "meaningCn": clean_seed_meaning_cn(&meaning_cn),
                "meaningEn": row.get::<_, Option<String>>(2)?
            }))
        })
        .map_err(|e| format!("Failed to query entry meaning detail: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode entry meaning detail row: {e}"))?;
    Ok(values)
}

fn load_entry_examples(
    conn: &rusqlite::Connection,
    entry_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT sentence_en, sentence_cn
             FROM entry_examples
             WHERE entry_id = ?1
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|e| format!("Failed to prepare entry examples query: {e}"))?;
    let values = stmt
        .query_map([entry_id], |row| {
            Ok(serde_json::json!({
                "sentenceEn": row.get::<_, String>(0)?,
                "sentenceCn": row.get::<_, String>(1)?
            }))
        })
        .map_err(|e| format!("Failed to query entry examples: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode entry examples row: {e}"))?;
    Ok(values)
}

fn recompute_summary_json(
    session_id: &str,
    results: &[serde_json::Value],
    completed_at: &str,
) -> Result<serde_json::Value, String> {
    let parsed_results = results
        .iter()
        .map(|value| {
            serde_json::from_value::<word_storage_core::models::StudyResult>(value.clone())
                .map_err(|e| format!("Failed to parse persisted study result: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let summary = word_storage_core::models::SessionSummary::from_results(
        session_id,
        &parsed_results,
        completed_at,
    );
    serde_json::to_value(summary)
        .map_err(|e| format!("Failed to serialize recomputed summary: {e}"))
}

pub fn build_reports_overview(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let today_date = request
        .get("todayDate")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let history = request
        .get("history")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let learned_count = request
        .get("learnedCount")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    let mut by_date = std::collections::BTreeMap::<String, serde_json::Value>::new();
    let mut by_mode = std::collections::BTreeMap::<String, serde_json::Value>::new();
    let mut by_mode_date = std::collections::BTreeMap::<
        String,
        std::collections::BTreeMap<String, serde_json::Value>,
    >::new();
    let mut study_days = std::collections::BTreeSet::<String>::new();
    let mut total_questions = 0.0f64;
    let mut total_correct = 0.0f64;

    for item in history {
        let date = item
            .get("date")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let mode = item
            .get("mode")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let Some(summary) = item.get("summary").and_then(|value| value.as_object()) else {
            continue;
        };
        study_days.insert(date.clone());
        total_questions += summary
            .get("totalQuestions")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        total_correct += summary
            .get("correctCount")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);

        let total_q = summary
            .get("totalQuestions")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let correct = summary
            .get("correctCount")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let time_ms = summary
            .get("totalTimeMs")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);

        let day = by_date.entry(date.clone()).or_insert_with(|| {
            serde_json::json!({
                "date": date,
                "totalQuestions": 0,
                "correctCount": 0,
                "studyTimeMs": 0
            })
        });
        day["totalQuestions"] = serde_json::json!(
            day.get("totalQuestions")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                + total_q
        );
        day["correctCount"] = serde_json::json!(
            day.get("correctCount")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                + correct
        );
        day["studyTimeMs"] = serde_json::json!(
            day.get("studyTimeMs")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                + time_ms
        );

        let mode_total = by_mode.entry(mode.clone()).or_insert_with(|| {
            serde_json::json!({
                "mode": mode,
                "totalQuestions": 0,
                "correctCount": 0
            })
        });
        mode_total["totalQuestions"] = serde_json::json!(
            mode_total
                .get("totalQuestions")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                + total_q
        );
        mode_total["correctCount"] = serde_json::json!(
            mode_total
                .get("correctCount")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                + correct
        );

        let mode_dates = by_mode_date.entry(mode).or_default();
        let mode_day = mode_dates.entry(date.clone()).or_insert_with(|| {
            serde_json::json!({
                "date": date,
                "totalQuestions": 0,
                "correctCount": 0,
                "studyTimeMs": 0
            })
        });
        mode_day["totalQuestions"] = serde_json::json!(
            mode_day
                .get("totalQuestions")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                + total_q
        );
        mode_day["correctCount"] = serde_json::json!(
            mode_day
                .get("correctCount")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                + correct
        );
        mode_day["studyTimeMs"] = serde_json::json!(
            mode_day
                .get("studyTimeMs")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
                + time_ms
        );
    }

    let last7_days = build_recent_days(&by_date, &today_date, 7);
    let daily_series = build_all_days_series(&by_date);
    let mode_breakdown = build_mode_breakdown(&by_mode);
    let mode_series = build_mode_series(&by_mode_date);
    let streak_info = build_streak_info(&study_days, &today_date);

    serde_json::to_string(&serde_json::json!({
        "totalStudyDays": study_days.len(),
        "totalWordsLearned": learned_count,
        "totalQuestionsAnswered": total_questions as u64,
        "overallAccuracy": if total_questions > 0.0 { (total_correct * 100.0) / total_questions } else { 0.0 },
        "streakInfo": streak_info,
        "modeBreakdown": mode_breakdown,
        "last7Days": last7_days,
        "dailySeries": daily_series,
        "modeSeries": mode_series
    }))
    .map_err(|e| format!("JSON serialization failed: {}", e))
}

pub fn build_wrong_words(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let filter = request
        .get("filter")
        .and_then(|value| value.as_str())
        .unwrap_or("all");
    let mut entries = request
        .get("entries")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    for entry in &mut entries {
        let error_count = entry
            .get("errorCount")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        let last_wrong_at = entry
            .get("lastWrongAt")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let priority = (error_count * 2.5) + (recency_score(last_wrong_at) * 0.2);
        entry["priorityScore"] = serde_json::json!(priority.min(10.0));
        if entry.get("isActive").is_none() {
            entry["isActive"] = serde_json::json!(true);
        }
    }

    entries.sort_by(|left, right| compare_wrong_word_entries(left, right, filter));
    if filter == "highPriority" {
        entries.retain(|entry| {
            entry
                .get("priorityScore")
                .and_then(|value| value.as_f64())
                .unwrap_or(0.0)
                >= 8.0
        });
    }

    serde_json::to_string(&entries).map_err(|e| format!("JSON serialization failed: {}", e))
}

pub fn build_wrong_word_detail(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    serde_json::to_string(&request).map_err(|e| format!("JSON serialization failed: {}", e))
}

pub fn build_today_ai_passage_context(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let tasks_complete = request
        .get("snapshot")
        .and_then(|value| value.as_object())
        .map(|snapshot| {
            let new_done = snapshot
                .get("newWordsCompleted")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let new_target = snapshot
                .get("newWordsTarget")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let review_done = snapshot
                .get("reviewWordsCompleted")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let review_target = snapshot
                .get("reviewWordsTarget")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let mixed_done = snapshot
                .get("mixedTestCompleted")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let mixed_target = snapshot
                .get("mixedTestTarget")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let wrong_done = snapshot
                .get("wrongWordTestCompleted")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let wrong_target = snapshot
                .get("wrongWordTestTarget")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let root_done = snapshot
                .get("rootAffixCompleted")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let root_target = snapshot
                .get("rootAffixTarget")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            new_done >= new_target
                && review_done >= review_target
                && mixed_done >= mixed_target
                && wrong_done >= wrong_target
                && root_done >= root_target
        })
        .unwrap_or(false);

    serde_json::to_string(&serde_json::json!({
        "date": request.get("date").and_then(|value| value.as_str()).unwrap_or(""),
        "tasksComplete": tasks_complete,
        "wrongWords": request.get("wrongWords").cloned().unwrap_or_else(|| serde_json::json!([]))
    }))
    .map_err(|e| format!("JSON serialization failed: {}", e))
}

pub fn get_active_plan() -> Result<String, String> {
    with_runtime_conn(|conn| {
        ensure_planning_state(conn)?;
        let value = get_json_setting(conn, "saved_plan_json", &default_plan_json())?;
        Ok(value.to_string())
    })
}

pub fn get_today_reward_state() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let today_date = today_date_string();
        let state = load_today_reward_state(conn, &today_date)?;
        Ok(state.to_string())
    })
}

pub fn draw_today_reward(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let reward_id = request
        .get("rewardId")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("rewardId is required")?;

    with_runtime_conn(|conn| {
        let today_date = today_date_string();
        let current = load_today_reward_state(conn, &today_date)?;
        if current
            .get("rewardId")
            .and_then(|value| value.as_str())
            .is_some_and(|value| !value.is_empty())
        {
            return Ok(current.to_string());
        }

        let state = serde_json::json!({
            "todayDate": today_date,
            "rewardId": reward_id,
            "claimedAt": chrono::Local::now().to_rfc3339(),
        });
        set_json_setting(conn, "today_reward_state_json", &state)?;
        Ok(state.to_string())
    })
}

pub fn save_plan(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let input = request
        .get("input")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    with_runtime_conn(|conn| {
        ensure_planning_state(conn)?;
        let mut plan = get_json_setting(conn, "saved_plan_json", &default_plan_json())?;
        let previous_plan = plan.clone();
        merge_json_object(&mut plan, &input);
        stamp_growth_rule_start_dates(&previous_plan, &input, &mut plan);
        set_json_setting(conn, "saved_plan_json", &plan)?;
        let outbox_payload = serde_json::json!({
            "type": "plan_config_snapshot",
            "plan": plan,
        });
        enqueue_sync_snapshot(
            conn,
            "plan_config",
            &outbox_payload,
            "plan_config:saved_plan_json",
        );
        Ok(plan.to_string())
    })
}

fn enqueue_sync_snapshot(
    conn: &word_storage_core::Connection,
    domain: &str,
    payload: &serde_json::Value,
    idempotency_key: &str,
) {
    let payload_json = payload.to_string();
    if let Err(error) = persistence::sync_repo::enqueue_latest_outbox_item(
        conn,
        domain,
        &payload_json,
        idempotency_key,
    ) {
        let _ = persistence::sync_repo::record_dead_letter(
            conn,
            domain,
            &payload_json,
            idempotency_key,
            "enqueue_failed",
            &error.to_string(),
        );
    }
}

#[derive(Default)]
struct StudyWordPointAggregate {
    point_date: String,
    entry_id: i64,
    mode: String,
    question_type: String,
    attempt_count: u32,
    correct_count: u32,
    wrong_count: u32,
    total_response_time_ms: u64,
    last_answered_at: String,
}

fn enqueue_recent_study_word_points(conn: &word_storage_core::Connection) {
    match build_recent_study_word_points_payload(conn, 35) {
        Ok(Some(payload)) => enqueue_sync_snapshot(
            conn,
            "study_word_points",
            &payload,
            "study_word_points:rolling_35_days",
        ),
        Ok(None) => {}
        Err(error) => {
            let payload = serde_json::json!({
                "type": "study_word_points_snapshot",
                "error": error,
            });
            let _ = persistence::sync_repo::record_dead_letter(
                conn,
                "study_word_points",
                &payload.to_string(),
                "study_word_points:rolling_35_days",
                "snapshot_failed",
                payload
                    .get("error")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
            );
        }
    }
}

fn enqueue_wrong_word_entries_snapshot(conn: &word_storage_core::Connection) {
    match build_wrong_word_entries_payload(conn) {
        Ok(Some(payload)) => enqueue_sync_snapshot(
            conn,
            "wrong_word_entries",
            &payload,
            "wrong_word_entries:current_snapshot",
        ),
        Ok(None) => {}
        Err(error) => {
            let payload = serde_json::json!({
                "type": "wrong_word_entries_snapshot",
                "error": error,
            });
            let _ = persistence::sync_repo::record_dead_letter(
                conn,
                "wrong_word_entries",
                &payload.to_string(),
                "wrong_word_entries:current_snapshot",
                "snapshot_failed",
                payload
                    .get("error")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
            );
        }
    }
}

fn build_wrong_word_entries_payload(
    conn: &word_storage_core::Connection,
) -> Result<Option<serde_json::Value>, String> {
    let mut entries = load_wrong_word_entries(conn)?;
    let entries = wrong_words_domain::build_wrong_words(&mut entries, "all")
        .into_iter()
        .filter_map(|entry| {
            let entry_id = entry.get("entryId").and_then(json_i64_value)?;
            let error_count = entry
                .get("errorCount")
                .and_then(json_i64_value)
                .unwrap_or(0);
            let last_wrong_at = entry
                .get("lastWrongAt")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if entry_id <= 0 || error_count <= 0 || last_wrong_at.is_empty() {
                return None;
            }
            let priority_score = entry
                .get("priorityScore")
                .and_then(|value| value.as_f64())
                .unwrap_or(error_count as f64);
            Some(serde_json::json!({
                "entryId": entry_id,
                "errorCount": error_count,
                "lastWrongAt": last_wrong_at,
                "priorityScore": priority_score,
                "hintText": entry
                    .get("userHint")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .trim(),
                "hintSource": entry
                    .get("hintSource")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                "hintUpdatedAt": entry
                    .get("hintUpdatedAt")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                "projectionVersion": 1,
            }))
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        return Ok(None);
    }

    Ok(Some(serde_json::json!({
        "type": "wrong_word_entries_snapshot",
        "generatedAt": chrono::Local::now().to_rfc3339(),
        "entries": entries,
    })))
}

fn enqueue_ai_passages_snapshot(conn: &word_storage_core::Connection) -> Result<usize, String> {
    let mut history = get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
    clean_seed_meaning_noise_in_json(&mut history);
    let mut enqueued = 0usize;
    for passage in history.as_array().cloned().unwrap_or_default() {
        let passage_id = passage
            .get("passageId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if passage_id.is_empty() {
            continue;
        }
        enqueue_sync_snapshot(
            conn,
            "ai_passages",
            &serde_json::json!({
                "type": "ai_passage_snapshot",
                "passage": passage,
            }),
            &format!("ai_passages:{passage_id}"),
        );
        enqueued = enqueued.saturating_add(1);
    }
    Ok(enqueued)
}

fn build_recent_study_word_points_payload(
    conn: &word_storage_core::Connection,
    window_days: i64,
) -> Result<Option<serde_json::Value>, String> {
    let cutoff_date = (chrono::Local::now().date_naive()
        - chrono::Duration::days(window_days.saturating_sub(1)))
    .format("%Y-%m-%d")
    .to_string();

    let mut stmt = conn
        .prepare(
            "SELECT s.mode, r.entry_id, r.question_type, r.outcome,
                    r.response_time_ms, r.answered_at
             FROM study_results r
             JOIN study_sessions s ON s.session_id = r.session_id
             WHERE substr(r.answered_at, 1, 10) >= ?1
             ORDER BY r.answered_at ASC, r.id ASC",
        )
        .map_err(|e| format!("Failed to prepare study word point query: {e}"))?;

    let rows = stmt
        .query_map([cutoff_date.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| format!("Failed to query study word point rows: {e}"))?;

    let mut aggregates = BTreeMap::<(String, i64, String, String), StudyWordPointAggregate>::new();

    for row in rows {
        let (mode, entry_id, question_type, outcome, response_time_ms, answered_at) =
            row.map_err(|e| format!("Failed to decode study word point row: {e}"))?;
        let point_date = local_date_from_rfc3339(&answered_at)
            .unwrap_or_else(|| answered_at.get(0..10).unwrap_or(&answered_at).to_string());
        if point_date.is_empty() || point_date < cutoff_date {
            continue;
        }

        let mode = normalize_persisted_enum_text(&mode);
        let question_type = normalize_persisted_enum_text(&question_type);
        let outcome = normalize_persisted_enum_text(&outcome);
        let key = (
            point_date.clone(),
            entry_id,
            mode.clone(),
            question_type.clone(),
        );
        let aggregate = aggregates
            .entry(key)
            .or_insert_with(|| StudyWordPointAggregate {
                point_date,
                entry_id,
                mode,
                question_type,
                ..Default::default()
            });

        aggregate.attempt_count = aggregate.attempt_count.saturating_add(1);
        if outcome == "correct" || outcome == "fuzzyCorrect" {
            aggregate.correct_count = aggregate.correct_count.saturating_add(1);
        } else if outcome == "incorrect" || outcome == "skipped" {
            aggregate.wrong_count = aggregate.wrong_count.saturating_add(1);
        }
        aggregate.total_response_time_ms = aggregate
            .total_response_time_ms
            .saturating_add(response_time_ms.max(0) as u64);
        if aggregate.last_answered_at.is_empty() || answered_at > aggregate.last_answered_at {
            aggregate.last_answered_at = answered_at;
        }
    }

    if aggregates.is_empty() {
        return Ok(None);
    }

    let points = aggregates
        .into_values()
        .map(|point| {
            serde_json::json!({
                "pointDate": point.point_date,
                "entryId": point.entry_id,
                "mode": point.mode,
                "questionType": point.question_type,
                "attemptCount": point.attempt_count,
                "correctCount": point.correct_count,
                "wrongCount": point.wrong_count,
                "totalResponseTimeMs": point.total_response_time_ms,
                "lastAnsweredAt": point.last_answered_at,
            })
        })
        .collect::<Vec<_>>();

    Ok(Some(serde_json::json!({
        "type": "study_word_points_snapshot",
        "windowDays": window_days,
        "generatedAt": chrono::Local::now().to_rfc3339(),
        "points": points,
    })))
}

pub fn apply_saved_plan_to_today() -> Result<String, String> {
    with_runtime_conn(|conn| {
        ensure_planning_state(conn)?;
        let today_date = today_date_string();
        let saved_plan = get_json_setting(conn, "saved_plan_json", &default_plan_json())?;
        let saved_wordbooks = get_json_setting(
            conn,
            "saved_wordbooks_json",
            &default_wordbook_selection_json(),
        )?;
        set_json_setting(conn, "today_plan_json", &saved_plan)?;
        set_json_setting(conn, "today_wordbooks_json", &saved_wordbooks)?;
        if should_replace_today_review_wordbooks(conn, &saved_wordbooks)? {
            set_json_setting(conn, "today_review_wordbooks_json", &saved_wordbooks)?;
        }
        persist_today_target_seed(conn, &today_date, &saved_plan)?;
        clear_persisted_active_study_sessions(conn)?;
        core_clear_all_active_sessions();
        Ok(saved_plan.to_string())
    })
}

fn clear_persisted_active_study_sessions(
    conn: &word_storage_core::Connection,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM app_settings WHERE key LIKE 'active_study_session_%'",
        [],
    )
    .map_err(|e| format!("Failed to clear active study sessions after applying plan: {e}"))?;
    Ok(())
}

fn should_replace_today_review_wordbooks(
    conn: &word_storage_core::Connection,
    next_wordbooks: &serde_json::Value,
) -> Result<bool, String> {
    let next_ids = active_wordbook_ids_from_selection(next_wordbooks);
    if next_ids.is_empty() {
        return Ok(true);
    }
    has_review_candidates_for_wordbooks(conn, &next_ids)
}

fn has_review_candidates_for_wordbooks(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
) -> Result<bool, String> {
    Ok(!load_learned_entry_ids_for_wordbooks(conn, wordbook_ids, 1)?.is_empty())
}

pub fn get_wordbooks() -> Result<String, String> {
    with_runtime_conn(|conn| {
        ensure_planning_state(conn)?;
        let selection = get_json_setting(
            conn,
            "saved_wordbooks_json",
            &default_wordbook_selection_json(),
        )?;
        let wordbooks = assemble_wordbooks(&selection);
        serde_json::to_string(&wordbooks).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn toggle_wordbook(wordbook_id: i64, is_active: bool) -> Result<(), String> {
    with_runtime_conn(|conn| {
        ensure_planning_state(conn)?;
        let selection = single_wordbook_selection_json(wordbook_id, is_active);
        set_json_setting(conn, "saved_wordbooks_json", &selection)?;
        let outbox_payload = serde_json::json!({
            "type": "wordbook_preferences_snapshot",
            "selection": selection,
        });
        enqueue_sync_snapshot(
            conn,
            "wordbook_preferences",
            &outbox_payload,
            "wordbook_preferences:saved_wordbooks_json",
        );
        Ok(())
    })
}

fn single_wordbook_selection_json(wordbook_id: i64, is_active: bool) -> serde_json::Value {
    let mut selection = default_wordbook_selection_json();
    if let Some(object) = selection.as_object_mut() {
        for value in object.values_mut() {
            *value = serde_json::Value::Bool(false);
        }
        if is_active {
            object.insert(wordbook_id.to_string(), serde_json::Value::Bool(true));
        }
        if !is_active {
            object.insert("1".to_string(), serde_json::Value::Bool(true));
        }
    }
    selection
}

pub fn save_ai_passage(request_json: String) -> Result<(), String> {
    let passage: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    with_runtime_conn(|conn| {
        let mut history =
            get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
        let history_array = history
            .as_array_mut()
            .ok_or("ai_passage_history_json is not an array")?;
        let passage_id = passage
            .get("passageId")
            .and_then(|value| value.as_str())
            .ok_or("passageId is required")?
            .to_string();
        history_array.retain(|item| {
            item.get("passageId")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                != passage_id
        });
        let mut passage = passage;
        clean_seed_meaning_noise_in_json(&mut passage);
        let outbox_payload = serde_json::json!({
            "type": "ai_passage_snapshot",
            "passage": passage.clone(),
        });
        let idempotency_key = format!("ai_passages:{}", passage_id);
        history_array.insert(0, passage);
        clean_seed_meaning_noise_in_json(&mut history);
        set_json_setting(conn, "ai_passage_history_json", &history)?;
        enqueue_sync_snapshot(conn, "ai_passages", &outbox_payload, &idempotency_key);
        Ok(())
    })
}

pub fn get_ai_passage_history() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let mut history =
            get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
        clean_seed_meaning_noise_in_json(&mut history);
        set_json_setting(conn, "ai_passage_history_json", &history)?;
        let items = history
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|item| {
                serde_json::json!({
                    "passageId": item.get("passageId").and_then(|value| value.as_str()).unwrap_or(""),
                    "title": item.get("title").and_then(|value| value.as_str()).unwrap_or(""),
                    "preview": item.get("preview").and_then(|value| value.as_str()).unwrap_or(""),
                    "wordCount": item.get("wordCount").and_then(|value| value.as_u64()).unwrap_or(0),
                    "date": item.get("date").and_then(|value| value.as_str()).unwrap_or(""),
                    "generatedAt": item.get("generatedAt").and_then(|value| value.as_str()).unwrap_or(""),
                    "validationStatus": item.get("validationStatus").and_then(|value| value.as_str()).unwrap_or("pending")
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_string(&items).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn get_ai_passage(passage_id: String) -> Result<String, String> {
    with_runtime_conn(|conn| {
        let mut history =
            get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
        clean_seed_meaning_noise_in_json(&mut history);
        set_json_setting(conn, "ai_passage_history_json", &history)?;
        let item = history
            .as_array()
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("passageId")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        == passage_id
                })
            })
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        serde_json::to_string(&item).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

fn find_ai_passage_for_date(
    conn: &word_storage_core::Connection,
    date: &str,
) -> Result<Option<serde_json::Value>, String> {
    let history = get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
    let existing = history.as_array().and_then(|items| {
        items
            .iter()
            .find(|item| ai_passage_matches_date(item, date))
            .cloned()
    });
    Ok(existing)
}

fn ai_passage_matches_date(item: &serde_json::Value, date: &str) -> bool {
    if date.is_empty() {
        return false;
    }
    item.get("date")
        .and_then(|value| value.as_str())
        .filter(|stored_date| *stored_date == date)
        .is_some()
        || item
            .get("generatedAt")
            .and_then(|value| value.as_str())
            .map(|generated_at| generated_at.starts_with(date))
            .unwrap_or(false)
}

pub fn analyze_wrong_word_import(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let source_type = request
        .get("sourceType")
        .and_then(|value| value.as_str())
        .unwrap_or("text");
    let source_name = request
        .get("sourceName")
        .and_then(|value| value.as_str())
        .unwrap_or(source_type);
    let text_content = request
        .get("textContent")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let bytes_base64 = request
        .get("bytesBase64")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let mime_type = request
        .get("mimeType")
        .and_then(|value| value.as_str())
        .unwrap_or("image/jpeg");

    if source_type == "image" {
        if bytes_base64.trim().is_empty() {
            let payload = serde_json::json!({
                "batchId": format!("preview_{}", chrono::Utc::now().timestamp_millis()),
                "sourceType": source_type,
                "sourceName": source_name,
                "warnings": ["No image bytes were received for AI analysis."],
                "candidates": []
            });
            return serde_json::to_string(&payload)
                .map_err(|e| format!("JSON serialization failed: {}", e));
        }

        let raw_content = analyze_wrong_word_image_with_ai(source_name, mime_type, bytes_base64)?;
        let parsed = parse_wrong_word_import_ai_output(&raw_content, source_type, source_name)?;
        return serde_json::to_string(&parsed)
            .map_err(|e| format!("JSON serialization failed: {}", e));
    }

    let mut counts = BTreeMap::<String, i64>::new();
    for word in extract_import_preview_words(text_content) {
        *counts.entry(word).or_insert(0) += 1;
    }

    let candidates = counts
        .into_iter()
        .map(|(word, count)| {
            serde_json::json!({
                "candidateId": word,
                "word": word,
                "meaning": null,
                "occurrenceCount": count,
                "confidence": if source_type == "image" { 0.72 } else { 0.86 },
                "isDuplicate": false,
                "isHighFrequency": count >= 2,
                "evidence": if count >= 2 {
                    format!("Appears {count} times in the imported source")
                } else {
                    "Appears once in the imported source".to_string()
                }
            })
        })
        .collect::<Vec<_>>();

    let payload = serde_json::json!({
        "batchId": format!("preview_{}", chrono::Utc::now().timestamp_millis()),
        "sourceType": source_type,
        "sourceName": source_name,
        "warnings": if candidates.is_empty() {
            serde_json::json!([
                if source_type == "image" {
                    "AI did not find any wrong-word candidates in this image. Try a clearer photo or crop closer to the notebook content."
                } else {
                    "No candidate words were found in the imported source."
                }
            ])
        } else {
            serde_json::json!([])
        },
        "candidates": candidates
    });
    serde_json::to_string(&payload).map_err(|e| format!("JSON serialization failed: {}", e))
}

fn analyze_wrong_word_image_with_ai(
    source_name: &str,
    mime_type: &str,
    bytes_base64: &str,
) -> Result<String, String> {
    let system_message = WRONG_WORD_IMPORT_PROMPT_TEMPLATE;
    let user_message = build_wrong_word_image_user_prompt(source_name, mime_type)?;
    ai_agent().run_image_json(
        system_message,
        &user_message,
        ImageInput {
            mime_type,
            bytes_base64,
        },
    )
}

fn build_wrong_word_image_user_prompt(
    source_name: &str,
    mime_type: &str,
) -> Result<String, String> {
    let metadata = serde_json::to_string_pretty(&serde_json::json!({
        "sourceType": "image",
        "sourceName": source_name,
        "mimeType": mime_type,
    }))
    .map_err(|e| format!("Failed to serialize import metadata: {e}"))?;
    Ok(format!(
        "Analyze the attached wrong-word notebook image.\n\n## Source Metadata\n```json\n{metadata}\n```\n\nReturn only the JSON object required by the system prompt."
    ))
}

fn parse_wrong_word_import_ai_output(
    content: &str,
    source_type: &str,
    source_name: &str,
) -> Result<serde_json::Value, String> {
    let cleaned = extract_json_payload(&strip_code_fences(content));
    let raw: serde_json::Value =
        serde_json::from_str(&cleaned).map_err(|e| format!("AI import JSON decode failed: {e}"))?;
    if raw.get("failed").and_then(|value| value.as_bool()) == Some(true) {
        return Err(raw
            .get("reason")
            .and_then(|value| value.as_str())
            .unwrap_or("AI wrong-word import failed")
            .to_string());
    }

    let candidates = raw
        .get("candidates")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(normalize_wrong_word_import_candidate)
        .collect::<Vec<_>>();
    let warnings = raw
        .get("warnings")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(str::trim).map(str::to_string))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "batchId": raw
            .get("batchId")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("ai_import_{}", chrono::Utc::now().timestamp_millis())),
        "sourceType": raw
            .get("sourceType")
            .and_then(|value| value.as_str())
            .unwrap_or(source_type),
        "sourceName": raw
            .get("sourceName")
            .and_then(|value| value.as_str())
            .unwrap_or(source_name),
        "warnings": warnings,
        "candidates": candidates,
    }))
}

fn normalize_wrong_word_import_candidate(value: serde_json::Value) -> Option<serde_json::Value> {
    let object = value.as_object()?;
    let word = object
        .get("word")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let candidate_id = object
        .get("candidateId")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(word);
    let occurrence_count = object
        .get("occurrenceCount")
        .and_then(|value| value.as_i64())
        .unwrap_or(1)
        .max(1);
    let confidence = object
        .get("confidence")
        .and_then(|value| value.as_f64())
        .unwrap_or(0.65)
        .clamp(0.0, 1.0);
    let evidence = object
        .get("evidence")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Detected by AI import analysis");

    Some(serde_json::json!({
        "candidateId": candidate_id,
        "word": word,
        "meaning": object.get("meaning").and_then(|value| value.as_str()).map(str::trim).filter(|value| !value.is_empty()),
        "occurrenceCount": occurrence_count,
        "confidence": confidence,
        "isDuplicate": object.get("isDuplicate").and_then(|value| value.as_bool()).unwrap_or(false),
        "isHighFrequency": object.get("isHighFrequency").and_then(|value| value.as_bool()).unwrap_or(occurrence_count >= 2),
        "evidence": evidence,
    }))
}

pub fn commit_wrong_word_import(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    with_runtime_conn(|conn| {
        let result = commit_wrong_word_import_with_connection(conn, &request)?;
        serde_json::to_string(&result).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

fn commit_wrong_word_import_with_connection(
    conn: &rusqlite::Connection,
    request: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let batch_id = request
        .get("batchId")
        .and_then(|value| value.as_str())
        .unwrap_or("manual-import");
    let source_type = request
        .get("sourceType")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    let source_name = request
        .get("sourceName")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    let accepted_ids = request
        .get("acceptedCandidateIds")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(|word| word.trim().to_string()))
        .filter(|word| !word.is_empty())
        .collect::<std::collections::BTreeSet<_>>();

    let candidate_source = request
        .get("candidates")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let candidates = if candidate_source.is_empty() {
        accepted_ids
            .iter()
            .map(|word| {
                serde_json::json!({
                    "candidateId": word,
                    "word": word,
                    "occurrenceCount": 1,
                    "confidence": 0.0,
                    "evidence": "Accepted import"
                })
            })
            .collect::<Vec<_>>()
    } else {
        candidate_source
    };

    let mut added = Vec::new();
    let mut skipped = Vec::new();
    let mut high_frequency = Vec::new();
    for candidate in candidates {
        let candidate_id = candidate
            .get("candidateId")
            .and_then(|value| value.as_str())
            .unwrap_or_else(|| {
                candidate
                    .get("word")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
            })
            .trim()
            .to_string();
        if !accepted_ids.contains(&candidate_id) {
            continue;
        }
        let word = candidate
            .get("word")
            .and_then(|value| value.as_str())
            .unwrap_or(&candidate_id)
            .trim();
        if word.is_empty() {
            continue;
        }
        let meaning = candidate
            .get("meaning")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let occurrence_count = candidate
            .get("occurrenceCount")
            .and_then(|value| value.as_i64())
            .unwrap_or(1)
            .max(1);
        let confidence = candidate
            .get("confidence")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0);
        let evidence = candidate
            .get("evidence")
            .and_then(|value| value.as_str())
            .unwrap_or("Accepted import");
        let is_high_frequency = candidate
            .get("isHighFrequency")
            .and_then(|value| value.as_bool())
            .unwrap_or(occurrence_count >= 3);
        let entry_id = find_entry_id_by_word(conn, word)?;

        let inserted = conn
            .execute(
                "INSERT OR IGNORE INTO imported_wrong_words
                 (batch_id, candidate_id, source_type, source_name, entry_id, word, meaning,
                  occurrence_count, confidence, evidence, is_high_frequency)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                rusqlite::params![
                    batch_id,
                    candidate_id,
                    source_type,
                    source_name,
                    entry_id,
                    word,
                    meaning,
                    occurrence_count,
                    confidence,
                    evidence,
                    if is_high_frequency { 1 } else { 0 },
                ],
            )
            .map_err(|e| format!("Failed to insert imported wrong word: {e}"))?;

        let payload = serde_json::json!({
            "candidateId": candidate_id,
            "word": word,
            "meaning": if meaning.is_empty() { serde_json::Value::Null } else { serde_json::json!(meaning) },
            "occurrenceCount": occurrence_count,
            "confidence": confidence,
            "isDuplicate": inserted == 0,
            "isHighFrequency": is_high_frequency,
            "evidence": evidence,
        });
        if inserted == 0 {
            skipped.push(payload);
        } else {
            if is_high_frequency {
                high_frequency.push(payload.clone());
            }
            added.push(payload);
        }
    }

    Ok(serde_json::json!({
        "persisted": true,
        "added": added,
        "skipped": skipped,
        "highFrequency": high_frequency
    }))
}

fn find_entry_id_by_word(conn: &rusqlite::Connection, word: &str) -> Result<Option<i64>, String> {
    conn.query_row(
        "SELECT id FROM entries WHERE LOWER(word) = LOWER(?1) ORDER BY frequency DESC, id ASC LIMIT 1",
        [word],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(|e| format!("Failed to match imported word to entry: {e}"))
}

fn extract_import_preview_words(text: &str) -> Vec<String> {
    text.split(|ch: char| !(ch.is_ascii_alphabetic() || ch == '-'))
        .map(|word| word.trim_matches('-').trim().to_ascii_lowercase())
        .filter(|word| word.len() >= 3)
        .take(24)
        .collect()
}

pub fn generate_ai_passage(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let date = request
        .get("date")
        .and_then(|value| value.as_str())
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .unwrap_or_else(today_date_string);
    let level = request
        .get("level")
        .and_then(|value| value.as_str())
        .unwrap_or("intermediate")
        .to_string();
    if let Some(existing) = with_runtime_conn(|conn| find_ai_passage_for_date(conn, &date))? {
        return serde_json::to_string(&existing)
            .map_err(|e| format!("JSON serialization failed: {}", e));
    }

    let wrong_words = resolve_ai_request_wrong_words(&request)?;
    if wrong_words.is_empty() {
        return Err("No wrong words available for passage generation".to_string());
    }

    let style = "default";
    let system_message =
        format!("{PROMPT_TEMPLATE}\n\n## Active Style\n\n{DEFAULT_STYLE_TEMPLATE}");
    let user_message = build_ai_user_prompt(&wrong_words, style, &date)?;

    let raw_content = ai_agent().run_text_json(&system_message, &user_message)?;
    let parsed = parse_ai_model_output(&raw_content, &wrong_words)?;
    let generated_at = chrono::Utc::now().to_rfc3339();
    let passage_id = format!("passage_{}", chrono::Utc::now().timestamp_millis());
    let blocks = parsed
        .get("blocks")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let covered_word_ids = parsed
        .get("coveredWordIds")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let missing_word_ids = parsed
        .get("missingWordIds")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let preview = build_ai_preview(&blocks);

    let passage = serde_json::json!({
        "passageId": passage_id,
        "title": parsed.get("title").and_then(|value| value.as_str()).filter(|value| !value.is_empty()).unwrap_or("AI 情境短文"),
        "blocks": blocks,
        "wrongWords": wrong_words,
        "coveredWordIds": covered_word_ids,
        "missingWordIds": missing_word_ids.clone(),
        "validationStatus": if missing_word_ids.is_empty() { "passed" } else { "failed" },
        "failureReason": if missing_word_ids.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::json!(format!("仍有 {} 个目标词未被覆盖。", missing_word_ids.len()))
        },
        "wordCount": estimate_ai_word_count_from_blocks(parsed.get("blocks").and_then(|value| value.as_array()).unwrap_or(&Vec::new())),
        "targetLevel": level,
        "date": date,
        "generatedAt": generated_at,
        "preview": preview,
    });

    save_ai_passage(passage.to_string())?;
    serde_json::to_string(&passage).map_err(|e| format!("JSON serialization failed: {}", e))
}

fn resolve_ai_request_wrong_words(
    request: &serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let wrong_words = request
        .get("wrongWords")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .take(AI_PASSAGE_MAX_WRONG_WORDS)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !wrong_words.is_empty() {
        return Ok(wrong_words);
    }

    let target_words = request
        .get("targetWords")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(|word| word.trim().to_ascii_lowercase()))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    with_runtime_conn(|conn| {
        let mut candidates = load_wrong_word_inputs(conn, 50)?;
        if !target_words.is_empty() {
            candidates.retain(|item| {
                item.get("word")
                    .and_then(|value| value.as_str())
                    .map(|word| target_words.contains(&word.trim().to_ascii_lowercase()))
                    .unwrap_or(false)
            });
        }
        candidates.truncate(AI_PASSAGE_MAX_WRONG_WORDS);
        Ok(candidates)
    })
}

fn compare_wrong_word_entries(
    left: &serde_json::Value,
    right: &serde_json::Value,
    filter: &str,
) -> std::cmp::Ordering {
    match filter {
        "recent" => right
            .get("lastWrongAt")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .cmp(
                left.get("lastWrongAt")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
            ),
        "frequent" => right
            .get("errorCount")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            .cmp(
                &left
                    .get("errorCount")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(0),
            ),
        _ => right
            .get("priorityScore")
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0)
            .partial_cmp(
                &left
                    .get("priorityScore")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0),
            )
            .unwrap_or(std::cmp::Ordering::Equal),
    }
}

const DEFAULT_PRIMARY_AI_URL: &str = "http://103.38.81.122:8080/v1/messages";
const DEFAULT_PRIMARY_AI_MODEL: &str = "claude";
const DEFAULT_PRIMARY_AI_KEY: &str =
    "sk-181b95262e1eaebe50313d416f5ed81e38164b7d71494f3e241f3149e63a3e31";
const DEFAULT_BACKUP_AI_URL: &str = "http://107.182.173.201:8080/v1/responses";
const DEFAULT_BACKUP_AI_MODEL: &str = "gpt-5.4";
const DEFAULT_BACKUP_AI_KEY: &str =
    "sk-d0fea41ec127dc71bbeb14da6a2507cc0bd7d92c740c512db67242d64358567d";
const AI_PASSAGE_MAX_WRONG_WORDS: usize = 24;
const PROMPT_TEMPLATE: &str = "You are a Chinese language learning assistant. Your task is to write a vivid, readable Chinese passage around specific English vocabulary words provided by the user.\n\nImportant: the backend will insert the actual English words and Chinese glosses. You should only decide where each word belongs inside the Chinese passage.\n\nYou will receive:\n- word_list: a JSON array of objects with word, primary_gloss, part_of_speech, entry_id\n- style: the desired writing style\n- length_target: approximate character count for the Chinese body text\n\nYou MUST return exactly one JSON object with this structure:\n{\"title\":\"Optional contextual title\",\"paragraphs\":[\"Chinese paragraph with markers such as [[word:101]] inside the text.\"]}\n\nRules:\n1. Return exactly one JSON object and nothing else.\n2. Use [[word:ENTRY_ID]] exactly once per target word.\n3. Do not output the English target words or glosses directly.\n4. Write natural Chinese paragraphs, not a word list.\n5. If there are many target words, write a longer passage with enough context for every word.\n6. Avoid default classroom or textbook scenes unless the words strongly require them.";
const DEFAULT_STYLE_TEMPLATE: &str = "Writing tone: imaginative, lively, and concrete while still easy to understand\nSentence length: Short to medium\nVocabulary level: Common Chinese vocabulary\nTopic connection: Use any fitting scene, such as travel, mystery, sci-fi, city life, dreams, myths, workplace drama, small adventures, or absurd comedy\nParagraph structure: adapt to the target word count\nCreativity: Prefer fresh situations over classroom explanations";
const WRONG_WORD_IMPORT_PROMPT_TEMPLATE: &str = "You are an English learning wrong-word import assistant. Analyze the user-provided source and extract only English vocabulary words that appear to be wrong words, missed words, correction targets, notebook entries, or repeated problem words.\n\nYou MUST return exactly one JSON object and nothing else:\n{\"sourceType\":\"image\",\"sourceName\":\"optional source name\",\"warnings\":[],\"candidates\":[{\"candidateId\":\"lowercase-word-or-stable-id\",\"word\":\"word\",\"meaning\":\"short Chinese meaning when visible or inferable, otherwise null\",\"occurrenceCount\":1,\"confidence\":0.0,\"isDuplicate\":false,\"isHighFrequency\":false,\"evidence\":\"brief evidence from the source\"}]}\n\nRules:\n1. Extract English words only. Do not invent words that are not visible or strongly implied.\n2. If handwriting or image quality is uncertain, lower confidence and add a warning.\n3. Mark isHighFrequency true when the same word appears multiple times, has an explicit count, or is visually emphasized as repeatedly wrong.\n4. Use occurrenceCount >= 1. Use confidence between 0 and 1.\n5. Keep evidence short and tied to visible source information.\n6. If no real candidate exists, return an empty candidates array with a warning.";

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredAiProviderProfile {
    provider: String,
    base_url: String,
    model: String,
    auth_token: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredAiProviderConfig {
    primary: StoredAiProviderProfile,
    backup: StoredAiProviderProfile,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AiProviderProfileSummary {
    provider: String,
    base_url: String,
    model: String,
    has_auth_token: bool,
    auth_token_preview: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct AiProviderConfigSummary {
    primary: AiProviderProfileSummary,
    backup: AiProviderProfileSummary,
}

fn build_ai_user_prompt(
    wrong_words: &[serde_json::Value],
    style: &str,
    date: &str,
) -> Result<String, String> {
    let word_list_json = serde_json::to_string_pretty(
        &wrong_words
            .iter()
            .map(|item| {
                serde_json::json!({
                    "word": item.get("word").and_then(|value| value.as_str()).unwrap_or(""),
                    "primary_gloss": item.get("primaryGloss").and_then(|value| value.as_str()).unwrap_or(""),
                    "part_of_speech": item.get("partOfSpeech").and_then(|value| value.as_str()).unwrap_or(""),
                    "entry_id": item.get("entryId").and_then(|value| value.as_i64()).unwrap_or(0),
                })
            })
            .collect::<Vec<_>>(),
    )
    .map_err(|e| format!("Failed to serialize AI prompt words: {}", e))?;

    let length_target = ai_passage_length_target(wrong_words.len());
    let paragraph_target = ai_passage_paragraph_target(wrong_words.len());

    Ok(format!(
        "Generate a Chinese passage plan for the following English vocabulary words.\n\n## Word List\n```json\n{word_list_json}\n```\n\n## Parameters\n- Style: {style}\n- Prompt version: 2.2\n- Target word count: {word_count}\n- Length target: {length_target}\n- Paragraph target: {paragraph_target}\n- Date: {date}\n\nRemember: write natural Chinese paragraphs and place each word exactly once using [[word:entry_id]] markers. Do not output the English words or glosses directly; the backend will inject them. Return only the JSON structure specified in the prompt template.",
        word_count = wrong_words.len(),
    ))
}

fn ai_passage_length_target(word_count: usize) -> &'static str {
    match word_count {
        0..=3 => "90-140 Chinese characters",
        4..=6 => "140-220 Chinese characters",
        7..=10 => "220-340 Chinese characters",
        11..=16 => "340-520 Chinese characters",
        _ => "520-760 Chinese characters",
    }
}

fn ai_passage_paragraph_target(word_count: usize) -> &'static str {
    match word_count {
        0..=6 => "2 paragraphs",
        7..=12 => "3 paragraphs",
        13..=18 => "4 paragraphs",
        _ => "5 paragraphs",
    }
}

fn read_non_empty_env(key: &str) -> Option<String> {
    env::var(key).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn default_ai_provider_config() -> StoredAiProviderConfig {
    StoredAiProviderConfig {
        primary: StoredAiProviderProfile {
            provider: "anthropic".to_string(),
            base_url: base_url_without_suffix(DEFAULT_PRIMARY_AI_URL, ANTHROPIC_MESSAGES_PATH),
            model: DEFAULT_PRIMARY_AI_MODEL.to_string(),
            auth_token: DEFAULT_PRIMARY_AI_KEY.to_string(),
        },
        backup: StoredAiProviderProfile {
            provider: "openaiResponses".to_string(),
            base_url: base_url_without_suffix(DEFAULT_BACKUP_AI_URL, OPENAI_RESPONSES_PATH),
            model: DEFAULT_BACKUP_AI_MODEL.to_string(),
            auth_token: DEFAULT_BACKUP_AI_KEY.to_string(),
        },
    }
}

fn redact_token(token: &str) -> String {
    if token.is_empty() {
        return String::new();
    }
    let visible_suffix: String = token
        .chars()
        .rev()
        .take(6)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("***{visible_suffix}")
}

fn summarize_ai_provider_config(config: &StoredAiProviderConfig) -> AiProviderConfigSummary {
    let to_summary = |profile: &StoredAiProviderProfile| AiProviderProfileSummary {
        provider: profile.provider.clone(),
        base_url: profile.base_url.clone(),
        model: profile.model.clone(),
        has_auth_token: !profile.auth_token.trim().is_empty(),
        auth_token_preview: redact_token(&profile.auth_token),
    };
    AiProviderConfigSummary {
        primary: to_summary(&config.primary),
        backup: to_summary(&config.backup),
    }
}

fn load_saved_ai_provider_config() -> Option<StoredAiProviderConfig> {
    let runtime_guard = get_runtime().ok()?;
    let runtime = runtime_guard.as_ref()?;
    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path).ok()?;
    let default_value = serde_json::to_value(default_ai_provider_config()).ok()?;
    get_json_setting(&conn, "ai_provider_config_json", &default_value)
        .ok()
        .and_then(|value| serde_json::from_value(value).ok())
}

fn resolve_ai_provider_config() -> StoredAiProviderConfig {
    let mut config = load_saved_ai_provider_config().unwrap_or_else(default_ai_provider_config);
    if let Some(base_url) = read_non_empty_env("ANTHROPIC_BASE_URL") {
        config.primary.base_url = base_url.trim_end_matches('/').to_string();
    }
    if let Some(model) = read_non_empty_env("ANTHROPIC_MODEL") {
        config.primary.model = model;
    }
    if let Some(token) = read_non_empty_env("ANTHROPIC_AUTH_TOKEN") {
        config.primary.auth_token = token;
    }
    if let Some(base_url) = read_non_empty_env("OPENAI_BASE_URL") {
        config.backup.base_url = base_url.trim_end_matches('/').to_string();
    }
    if let Some(model) = read_non_empty_env("OPENAI_MODEL") {
        config.backup.model = model;
    }
    if let Some(token) = read_non_empty_env("OPENAI_AUTH_TOKEN") {
        config.backup.auth_token = token;
    }
    config
}

fn ai_agent() -> AiAgent {
    let config = resolve_ai_provider_config();
    AiAgent::new(AiProviderConfig {
        primary: AiProviderProfile {
            provider: config.primary.provider,
            base_url: config.primary.base_url,
            model: config.primary.model,
            auth_token: config.primary.auth_token,
        },
        backup: AiProviderProfile {
            provider: config.backup.provider,
            base_url: config.backup.base_url,
            model: config.backup.model,
            auth_token: config.backup.auth_token,
        },
    })
}

fn parse_ai_model_output(
    content: &str,
    wrong_words: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    let cleaned = extract_json_payload(&strip_code_fences(content));
    let raw: serde_json::Value =
        serde_json::from_str(&cleaned).map_err(|e| format!("AI JSON decode failed: {e}"))?;
    if raw.get("failed").and_then(|value| value.as_bool()) == Some(true) {
        return Err(raw
            .get("reason")
            .and_then(|value| value.as_str())
            .unwrap_or("AI passage generation failed")
            .to_string());
    }
    let paragraphs = raw
        .get("paragraphs")
        .and_then(|value| value.as_array())
        .ok_or("AI response missing paragraphs")?;

    let mut lookup = std::collections::BTreeMap::<i64, serde_json::Value>::new();
    for item in wrong_words {
        let entry_id = item
            .get("entryId")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        lookup.insert(entry_id, item.clone());
    }
    let mut covered = std::collections::BTreeSet::<i64>::new();
    let marker = regex::Regex::new(r"\[\[word:(-?\d+)\]\]").map_err(|e| e.to_string())?;
    let mut blocks = Vec::new();

    for paragraph in paragraphs {
        let paragraph_text = paragraph.as_str().unwrap_or("");
        let mut segments = Vec::new();
        let mut cursor = 0usize;
        for capture in marker.captures_iter(paragraph_text) {
            let whole = capture.get(0).unwrap();
            if whole.start() > cursor {
                segments.push(serde_json::json!({
                    "type": "text",
                    "text": &paragraph_text[cursor..whole.start()]
                }));
            }
            let entry_id = capture
                .get(1)
                .and_then(|value| value.as_str().parse::<i64>().ok())
                .unwrap_or(0);
            let Some(word) = lookup.get(&entry_id) else {
                return Err(format!("AI returned unknown entryId: {entry_id}"));
            };
            covered.insert(entry_id);
            segments.push(serde_json::json!({
                "type": "word",
                "text": word.get("word").and_then(|value| value.as_str()).unwrap_or(""),
                "entryId": entry_id,
                "glossZh": word.get("primaryGloss").and_then(|value| value.as_str()).unwrap_or(""),
                "highlighted": true
            }));
            cursor = whole.end();
        }
        if cursor < paragraph_text.len() {
            segments.push(serde_json::json!({
                "type": "text",
                "text": &paragraph_text[cursor..]
            }));
        }
        blocks.push(serde_json::json!({
            "blockType": "paragraph",
            "segments": segments
        }));
    }

    let covered_word_ids = covered
        .iter()
        .map(|id| serde_json::json!(id))
        .collect::<Vec<_>>();
    let missing_word_ids = wrong_words
        .iter()
        .map(|item| {
            item.get("entryId")
                .and_then(|value| value.as_i64())
                .unwrap_or(0)
        })
        .filter(|entry_id| !covered.contains(entry_id))
        .map(|id| serde_json::json!(id))
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "title": raw.get("title").and_then(|value| value.as_str()).unwrap_or(""),
        "blocks": blocks,
        "coveredWordIds": covered_word_ids,
        "missingWordIds": missing_word_ids
    }))
}

fn estimate_ai_word_count_from_blocks(blocks: &[serde_json::Value]) -> usize {
    blocks
        .iter()
        .flat_map(|block| {
            block
                .get("segments")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .map(|segment| {
            segment
                .get("text")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .chars()
                .count()
        })
        .sum()
}

fn build_ai_preview(blocks: &[serde_json::Value]) -> String {
    let text = blocks
        .iter()
        .flat_map(|block| {
            block
                .get("segments")
                .and_then(|value| value.as_array())
                .cloned()
                .unwrap_or_default()
        })
        .map(|segment| {
            if segment.get("type").and_then(|value| value.as_str()) == Some("word") {
                let word = segment
                    .get("text")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                let gloss = segment
                    .get("glossZh")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                if gloss.is_empty() {
                    word.to_string()
                } else {
                    format!("{word}（{gloss}）")
                }
            } else {
                segment
                    .get("text")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string()
            }
        })
        .collect::<String>();
    if text.chars().count() > 80 {
        text.chars().take(80).collect::<String>() + "..."
    } else {
        text
    }
}

fn recency_score(last_wrong_at: &str) -> f64 {
    let date = last_wrong_at.get(0..10).unwrap_or(last_wrong_at);
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        let today = chrono::Local::now().date_naive();
        let days = (today - parsed).num_days().max(0);
        (21 - days.min(21)) as f64
    } else {
        0.0
    }
}

fn build_recent_days(
    by_date: &std::collections::BTreeMap<String, serde_json::Value>,
    today_date: &str,
    days: usize,
) -> Vec<serde_json::Value> {
    let end = chrono::NaiveDate::parse_from_str(today_date, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Local::now().date_naive());
    let mut result = Vec::new();
    for offset in (0..days).rev() {
        let date = end - chrono::Days::new(offset as u64);
        let key = date.format("%Y-%m-%d").to_string();
        let day = by_date.get(&key);
        let total_questions = day
            .and_then(|value| value.get("totalQuestions"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let correct_count = day
            .and_then(|value| value.get("correctCount"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let study_time_ms = day
            .and_then(|value| value.get("studyTimeMs"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        result.push(serde_json::json!({
            "date": key,
            "totalQuestions": total_questions,
            "correctCount": correct_count,
            "accuracyPercent": if total_questions > 0 { (correct_count as f64 * 100.0) / total_questions as f64 } else { 0.0 },
            "studyTimeMs": study_time_ms
        }));
    }
    result
}

fn build_all_days_series(
    by_date: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Vec<serde_json::Value> {
    by_date
        .iter()
        .map(|(date, day)| {
            let total_questions = day
                .get("totalQuestions")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let correct_count = day
                .get("correctCount")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let study_time_ms = day
                .get("studyTimeMs")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            serde_json::json!({
                "date": date,
                "totalQuestions": total_questions,
                "correctCount": correct_count,
                "accuracyPercent": if total_questions > 0 { (correct_count as f64 * 100.0) / total_questions as f64 } else { 0.0 },
                "studyTimeMs": study_time_ms
            })
        })
        .collect()
}

fn build_mode_breakdown(
    by_mode: &std::collections::BTreeMap<String, serde_json::Value>,
) -> Vec<serde_json::Value> {
    ["newWord", "review", "mixedTest", "wrongWordReinforcement", "rootAffix"]
        .iter()
        .map(|mode| {
            let mode_total = by_mode.get(*mode);
            let total_questions = mode_total
                .and_then(|value| value.get("totalQuestions"))
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let correct_count = mode_total
                .and_then(|value| value.get("correctCount"))
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            serde_json::json!({
                "mode": mode,
                "totalQuestions": total_questions,
                "correctCount": correct_count,
                "accuracyPercent": if total_questions > 0 { (correct_count as f64 * 100.0) / total_questions as f64 } else { 0.0 }
            })
        })
        .collect()
}

fn build_mode_series(
    by_mode_date: &std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, serde_json::Value>,
    >,
) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    for mode in [
        "newWord",
        "review",
        "mixedTest",
        "wrongWordReinforcement",
        "rootAffix",
    ] {
        let series = by_mode_date
            .get(mode)
            .map(build_all_days_series)
            .unwrap_or_default();
        out.insert(mode.to_string(), serde_json::Value::Array(series));
    }
    serde_json::Value::Object(out)
}

fn build_streak_info(
    study_days: &std::collections::BTreeSet<String>,
    today_date: &str,
) -> serde_json::Value {
    let ordered: Vec<_> = study_days.iter().cloned().collect();
    let mut longest = 0i64;
    let mut running = 0i64;
    let mut prev: Option<chrono::NaiveDate> = None;
    for date in &ordered {
        if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
            if let Some(prev_date) = prev {
                if (parsed - prev_date).num_days() == 1 {
                    running += 1;
                } else {
                    running = 1;
                }
            } else {
                running = 1;
            }
            longest = longest.max(running);
            prev = Some(parsed);
        }
    }
    let mut current = 0i64;
    let mut cursor = chrono::NaiveDate::parse_from_str(today_date, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Local::now().date_naive());
    loop {
        let key = cursor.format("%Y-%m-%d").to_string();
        if !study_days.contains(&key) {
            break;
        }
        current += 1;
        cursor -= chrono::TimeDelta::days(1);
    }
    serde_json::json!({
        "currentStreak": current,
        "longestStreak": longest,
        "lastStudyDate": ordered.last().cloned()
    })
}

fn with_runtime_conn<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce(&word_storage_core::Connection) -> Result<T, String>,
{
    with_runtime(|_, conn| f(conn))
}

fn with_runtime<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce(&MobileRuntime, &word_storage_core::Connection) -> Result<T, String>,
{
    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;
    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;
    f(runtime, &conn)
}

fn ensure_seed_vocabulary_imported(
    conn: &word_storage_core::Connection,
    bundle_dir: &Path,
) -> Result<(), String> {
    let existing_entries = conn
        .query_row("SELECT COUNT(*) FROM entries", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| format!("Failed to count vocabulary entries: {e}"))?;
    let existing_links = conn
        .query_row("SELECT COUNT(*) FROM wordbook_entries", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| format!("Failed to count wordbook entries: {e}"))?;
    if existing_entries > 0 && existing_links > 0 {
        return repair_seed_vocabulary_dedup(conn);
    }

    let book_dir = bundle_dir.join("seed-vocab").join("book");
    if !book_dir.exists() {
        return Ok(());
    }

    conn.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| format!("Failed to begin seed vocabulary import: {e}"))?;
    let result = import_seed_vocabulary(conn, &book_dir);
    match result {
        Ok(()) => conn
            .execute_batch("COMMIT")
            .map_err(|e| format!("Failed to commit seed vocabulary import: {e}"))
            .and_then(|_| repair_seed_vocabulary_dedup(conn)),
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

fn import_seed_vocabulary(
    conn: &word_storage_core::Connection,
    book_dir: &Path,
) -> Result<(), String> {
    let source_version_id = ensure_seed_source_version(conn)?;
    let books = [
        (1i64, "cet4", "CET-4", "CET4_3.json", "CET4_3", "exam"),
        (2i64, "cet6", "CET-6", "CET6_3.json", "CET6_3", "exam"),
        (
            3i64,
            "kaoyan",
            "KaoYan",
            "KaoYan_3.json",
            "KaoYan_3",
            "exam",
        ),
        (
            4i64,
            "medical",
            "Medical English",
            "MEDICAL_RESP.json",
            "MEDICAL_RESP",
            "specialized",
        ),
    ];

    for (book_id, code, name, file_name, source_book_id, category) in books {
        let path = book_dir.join(file_name);
        if !path.exists() {
            continue;
        }
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, category, source_book_id, source_version_id, total_entries, is_active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0)
             ON CONFLICT(id) DO UPDATE SET
                code = excluded.code,
                name = excluded.name,
                category = excluded.category,
                source_book_id = excluded.source_book_id,
                source_version_id = excluded.source_version_id",
            rusqlite::params![book_id, code, name, category, source_book_id, source_version_id],
        )
        .map_err(|e| format!("Failed to upsert seed wordbook {code}: {e}"))?;
        let imported = import_seed_book(conn, source_version_id, book_id, &path)?;
        conn.execute(
            "UPDATE wordbooks SET total_entries = ?1 WHERE id = ?2",
            rusqlite::params![imported as i64, book_id],
        )
        .map_err(|e| format!("Failed to update seed wordbook count {code}: {e}"))?;
    }

    Ok(())
}

fn ensure_seed_source_version(conn: &word_storage_core::Connection) -> Result<i64, String> {
    conn.execute(
        "INSERT INTO source_versions (source_name, source_commit, snapshot_path, status, notes)
         VALUES ('bundled-seed-vocab', 'bundled-seed-vocab-v1', 'seed-vocab/book', 'ready', 'Imported from bundled mobile seed-vocab JSON assets')
         ON CONFLICT(source_commit) DO UPDATE SET status = 'ready'",
        [],
    )
    .map_err(|e| format!("Failed to upsert seed source version: {e}"))?;
    conn.query_row(
        "SELECT id FROM source_versions WHERE source_commit = 'bundled-seed-vocab-v1'",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map_err(|e| format!("Failed to load seed source version id: {e}"))
}

fn import_seed_book(
    conn: &word_storage_core::Connection,
    source_version_id: i64,
    wordbook_id: i64,
    path: &Path,
) -> Result<usize, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read seed book {}: {e}", path.display()))?;
    let words: Vec<serde_json::Value> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse seed book {}: {e}", path.display()))?;

    let mut imported = 0usize;
    for item in words {
        let Some(word) = item
            .get("headWord")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let source_key = item
            .pointer("/content/word/wordId")
            .and_then(|value| value.as_str())
            .unwrap_or(word);
        let word_content = item.pointer("/content/word/content");
        let phonetic_us = word_content
            .and_then(|value| value.get("usphone"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let phonetic_uk = word_content
            .and_then(|value| value.get("ukphone"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let part_of_speech = word_content
            .and_then(|value| value.pointer("/trans/0/pos"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let rank = item
            .get("wordRank")
            .and_then(|value| value.as_i64())
            .unwrap_or(imported as i64 + 1);
        let frequency = 1_000_000.0 - rank.max(0) as f64;

        let canonical_entry_id =
            find_seed_entry_for_word_pos(conn, wordbook_id, word, part_of_speech)?;
        let (entry_id, should_replace_details) = if let Some(entry_id) = canonical_entry_id {
            conn.execute(
                "UPDATE entries
                 SET frequency = MAX(frequency, ?1),
                     phonetic_us = CASE WHEN COALESCE(phonetic_us, '') = '' THEN ?2 ELSE phonetic_us END,
                     phonetic_uk = CASE WHEN COALESCE(phonetic_uk, '') = '' THEN ?3 ELSE phonetic_uk END
                 WHERE id = ?4",
                rusqlite::params![frequency, phonetic_us, phonetic_uk, entry_id],
            )
            .map_err(|e| format!("Failed to merge seed entry {source_key}: {e}"))?;
            (entry_id, false)
        } else {
            conn.execute(
                "INSERT INTO entries (source_version_id, source_entry_key, word, lemma, phonetic_us, phonetic_uk, part_of_speech, frequency, difficulty)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, '')
                 ON CONFLICT(source_version_id, source_entry_key) DO UPDATE SET
                    word = excluded.word,
                    lemma = excluded.lemma,
                    phonetic_us = excluded.phonetic_us,
                    phonetic_uk = excluded.phonetic_uk,
                    part_of_speech = excluded.part_of_speech,
                    frequency = excluded.frequency",
                rusqlite::params![
                    source_version_id,
                    source_key,
                    word,
                    word.to_lowercase(),
                    phonetic_us,
                    phonetic_uk,
                    part_of_speech,
                    frequency,
                ],
            )
            .map_err(|e| format!("Failed to upsert seed entry {source_key}: {e}"))?;
            let entry_id = conn
                .query_row(
                    "SELECT id FROM entries WHERE source_version_id = ?1 AND source_entry_key = ?2",
                    rusqlite::params![source_version_id, source_key],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(|e| format!("Failed to reload seed entry {source_key}: {e}"))?;
            (entry_id, true)
        };

        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(wordbook_id, entry_id) DO UPDATE SET
                rank_in_book = MIN(wordbook_entries.rank_in_book, excluded.rank_in_book)",
            rusqlite::params![wordbook_id, entry_id, rank],
        )
        .map_err(|e| format!("Failed to link seed entry {source_key}: {e}"))?;

        if should_replace_details {
            replace_seed_entry_meanings(conn, entry_id, word_content)?;
            replace_seed_entry_examples(conn, entry_id, word_content)?;
        } else {
            append_seed_entry_meanings(conn, entry_id, word_content)?;
            append_seed_entry_examples(conn, entry_id, word_content)?;
        }
        imported += 1;
    }

    Ok(imported)
}

fn find_seed_entry_for_word_pos(
    conn: &word_storage_core::Connection,
    wordbook_id: i64,
    word: &str,
    part_of_speech: &str,
) -> Result<Option<i64>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.part_of_speech
             FROM wordbook_entries we
             JOIN entries e ON e.id = we.entry_id
             WHERE we.wordbook_id = ?1 AND LOWER(e.word) = LOWER(?2)
             ORDER BY we.rank_in_book ASC, e.id ASC",
        )
        .map_err(|e| format!("Failed to prepare duplicate seed entry query: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params![wordbook_id, word], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .map_err(|e| format!("Failed to query duplicate seed entry: {e}"))?;
    let target_pos = normalize_seed_pos_key(part_of_speech);
    for row in rows {
        let (entry_id, existing_pos) =
            row.map_err(|e| format!("Failed to decode duplicate seed entry row: {e}"))?;
        if normalize_seed_pos_key(existing_pos.as_deref().unwrap_or("")) == target_pos {
            return Ok(Some(entry_id));
        }
    }
    Ok(None)
}

fn replace_seed_entry_meanings(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    word_content: Option<&serde_json::Value>,
) -> Result<(), String> {
    conn.execute("DELETE FROM entry_meanings WHERE entry_id = ?1", [entry_id])
        .map_err(|e| format!("Failed to clear seed entry meanings: {e}"))?;
    append_seed_entry_meanings(conn, entry_id, word_content)
}

fn append_seed_entry_meanings(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    word_content: Option<&serde_json::Value>,
) -> Result<(), String> {
    let meanings = word_content
        .and_then(|value| value.get("trans"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    for (index, meaning) in meanings.iter().enumerate() {
        let meaning_cn = meaning
            .get("tranCn")
            .and_then(|value| value.as_str())
            .map(clean_seed_meaning_cn)
            .unwrap_or_default();
        if meaning_cn.is_empty() {
            continue;
        }
        let pos = meaning
            .get("pos")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let exists = conn
            .query_row(
                "SELECT 1 FROM entry_meanings
                 WHERE entry_id = ?1 AND pos = ?2 AND meaning_cn = ?3
                 LIMIT 1",
                rusqlite::params![entry_id, pos, meaning_cn],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| format!("Failed to check seed entry meaning: {e}"))?;
        if exists.is_some() {
            continue;
        }
        let sort_order = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM entry_meanings WHERE entry_id = ?1",
                [entry_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to compute seed entry meaning order: {e}"))?;
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, meaning_en, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                entry_id,
                pos,
                meaning_cn.as_str(),
                meaning.get("tranOther").and_then(|value| value.as_str()),
                sort_order.max(index as i64),
            ],
        )
        .map_err(|e| format!("Failed to insert seed entry meaning: {e}"))?;
    }
    Ok(())
}

fn replace_seed_entry_examples(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    word_content: Option<&serde_json::Value>,
) -> Result<(), String> {
    conn.execute("DELETE FROM entry_examples WHERE entry_id = ?1", [entry_id])
        .map_err(|e| format!("Failed to clear seed entry examples: {e}"))?;
    append_seed_entry_examples(conn, entry_id, word_content)
}

fn append_seed_entry_examples(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    word_content: Option<&serde_json::Value>,
) -> Result<(), String> {
    let examples = word_content
        .and_then(|value| value.pointer("/sentence/sentences"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    for (index, example) in examples.iter().take(3).enumerate() {
        let sentence_en = example
            .get("sContent")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        if sentence_en.is_empty() {
            continue;
        }
        let exists = conn
            .query_row(
                "SELECT 1 FROM entry_examples
                 WHERE entry_id = ?1 AND sentence_en = ?2
                 LIMIT 1",
                rusqlite::params![entry_id, sentence_en],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| format!("Failed to check seed entry example: {e}"))?;
        if exists.is_some() {
            continue;
        }
        let sort_order = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM entry_examples WHERE entry_id = ?1",
                [entry_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|e| format!("Failed to compute seed entry example order: {e}"))?;
        conn.execute(
            "INSERT INTO entry_examples (entry_id, sentence_en, sentence_cn, sort_order)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                entry_id,
                sentence_en,
                example
                    .get("sCn")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                sort_order.max(index as i64),
            ],
        )
        .map_err(|e| format!("Failed to insert seed entry example: {e}"))?;
    }
    Ok(())
}

fn normalize_seed_pos_key(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('.').to_ascii_lowercase();
    if trimmed.starts_with('v') {
        "v".to_string()
    } else if trimmed.starts_with('n') {
        "n".to_string()
    } else if trimmed.starts_with("adj") || trimmed == "a" {
        "adj".to_string()
    } else if trimmed.starts_with("adv") {
        "adv".to_string()
    } else if trimmed.starts_with("prep") {
        "prep".to_string()
    } else if trimmed.starts_with("conj") {
        "conj".to_string()
    } else if trimmed.starts_with("pron") {
        "pron".to_string()
    } else {
        trimmed
    }
}

fn clean_seed_meaning_cn(value: &str) -> String {
    let without_markers = value.replace(['<', '>'], "");
    let collapsed = without_markers
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    let normalized = collapsed
        .replace(" ；", "；")
        .replace("； ", "；")
        .replace(" ，", "，")
        .replace("， ", "，")
        .replace(" ：", "：")
        .replace("： ", "：");
    strip_seed_exam_markers(&normalized)
}

fn strip_seed_exam_markers(value: &str) -> String {
    let mut parts = value
        .split(['；', ';'])
        .map(|part| {
            part.trim()
                .trim_end_matches(|ch| matches!(ch, 'A' | 'B' | 'C' | 'D'))
                .trim()
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return String::new();
    }
    let joined = parts.join("；");
    parts.clear();
    joined
}

fn clean_seed_meaning_text_if_needed(value: &str) -> String {
    if contains_han(value) && (value.contains(['<', '>']) || has_seed_exam_marker(value)) {
        clean_seed_meaning_cn(value)
    } else {
        value.to_string()
    }
}

fn has_seed_exam_marker(value: &str) -> bool {
    value
        .split(['；', ';'])
        .any(|part| matches!(part.trim().chars().last(), Some('A' | 'B' | 'C' | 'D')))
}

fn clean_seed_meaning_noise_in_json(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => {
            let cleaned = clean_seed_meaning_text_if_needed(text);
            if cleaned != *text {
                *text = cleaned;
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                clean_seed_meaning_noise_in_json(item);
            }
        }
        serde_json::Value::Object(map) => {
            for child in map.values_mut() {
                clean_seed_meaning_noise_in_json(child);
            }
        }
        _ => {}
    }
}

fn clean_json_string<T: serde::Serialize>(value: &T) -> Result<String, String> {
    let mut json =
        serde_json::to_value(value).map_err(|e| format!("JSON serialization failed: {e}"))?;
    clean_seed_meaning_noise_in_json(&mut json);
    serde_json::to_string(&json).map_err(|e| format!("JSON serialization failed: {e}"))
}

fn repair_seed_vocabulary_dedup(conn: &word_storage_core::Connection) -> Result<(), String> {
    repair_seed_meaning_noise(conn)?;

    let mut stmt = conn
        .prepare(
            "SELECT we.wordbook_id, we.entry_id, we.rank_in_book, e.word, e.part_of_speech
             FROM wordbook_entries we
             JOIN entries e ON e.id = we.entry_id
             ORDER BY we.wordbook_id ASC, LOWER(e.word) ASC, we.rank_in_book ASC, e.id ASC",
        )
        .map_err(|e| format!("Failed to prepare seed dedup scan: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|e| format!("Failed to query seed dedup scan: {e}"))?;

    let mut canonical_by_key = BTreeMap::<(i64, String), (i64, i64)>::new();
    let mut merges = Vec::<(i64, i64, i64, i64)>::new();
    for row in rows {
        let (wordbook_id, entry_id, rank, word, _part_of_speech) =
            row.map_err(|e| format!("Failed to decode seed dedup row: {e}"))?;
        let key = (wordbook_id, word.trim().to_ascii_lowercase());
        if let Some((canonical_id, canonical_rank)) = canonical_by_key.get_mut(&key) {
            if rank < *canonical_rank {
                *canonical_rank = rank;
            }
            if *canonical_id != entry_id {
                merges.push((wordbook_id, entry_id, *canonical_id, rank));
            }
        } else {
            canonical_by_key.insert(key, (entry_id, rank));
        }
    }

    for (wordbook_id, duplicate_id, canonical_id, _rank) in merges {
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, meaning_en, sort_order)
             SELECT ?1, dm.pos, dm.meaning_cn, dm.meaning_en,
                    COALESCE((SELECT MAX(sort_order) FROM entry_meanings WHERE entry_id = ?1), -1) + dm.sort_order + 1
             FROM entry_meanings dm
             WHERE dm.entry_id = ?2
               AND NOT EXISTS (
                   SELECT 1 FROM entry_meanings cm
                   WHERE cm.entry_id = ?1 AND cm.pos = dm.pos AND cm.meaning_cn = dm.meaning_cn
               )",
            rusqlite::params![canonical_id, duplicate_id],
        )
        .map_err(|e| format!("Failed to merge duplicate meanings: {e}"))?;
        conn.execute(
            "INSERT INTO entry_examples (entry_id, sentence_en, sentence_cn, sort_order)
             SELECT ?1, de.sentence_en, de.sentence_cn,
                    COALESCE((SELECT MAX(sort_order) FROM entry_examples WHERE entry_id = ?1), -1) + de.sort_order + 1
             FROM entry_examples de
             WHERE de.entry_id = ?2
               AND NOT EXISTS (
                   SELECT 1 FROM entry_examples ce
                   WHERE ce.entry_id = ?1 AND ce.sentence_en = de.sentence_en
               )",
            rusqlite::params![canonical_id, duplicate_id],
        )
        .map_err(|e| format!("Failed to merge duplicate examples: {e}"))?;
        conn.execute(
            "UPDATE study_results SET entry_id = ?1 WHERE entry_id = ?2",
            rusqlite::params![canonical_id, duplicate_id],
        )
        .map_err(|e| format!("Failed to move duplicate study results: {e}"))?;
        conn.execute(
            "DELETE FROM wordbook_entries WHERE wordbook_id = ?1 AND entry_id = ?2",
            rusqlite::params![wordbook_id, duplicate_id],
        )
        .map_err(|e| format!("Failed to unlink duplicate wordbook entry: {e}"))?;
    }

    let mut wordbook_stmt = conn
        .prepare("SELECT id FROM wordbooks")
        .map_err(|e| format!("Failed to prepare wordbook recount: {e}"))?;
    let wordbook_ids = wordbook_stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query wordbook recount: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode wordbook recount: {e}"))?;
    for wordbook_id in wordbook_ids {
        conn.execute(
            "UPDATE wordbooks
             SET total_entries = (
                 SELECT COUNT(*) FROM wordbook_entries WHERE wordbook_id = ?1
             )
             WHERE id = ?1",
            [wordbook_id],
        )
        .map_err(|e| format!("Failed to update wordbook total after dedup: {e}"))?;
    }

    Ok(())
}

fn repair_seed_meaning_noise(conn: &word_storage_core::Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("SELECT id, meaning_cn FROM entry_meanings")
        .map_err(|e| format!("Failed to prepare seed meaning noise scan: {e}"))?;
    let repairs = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| format!("Failed to query seed meaning noise: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode seed meaning noise row: {e}"))?;
    drop(stmt);

    for (id, meaning_cn) in repairs {
        let cleaned = clean_seed_meaning_cn(&meaning_cn);
        if cleaned != meaning_cn {
            conn.execute(
                "UPDATE entry_meanings SET meaning_cn = ?1 WHERE id = ?2",
                rusqlite::params![cleaned, id],
            )
            .map_err(|e| format!("Failed to repair seed meaning noise: {e}"))?;
        }
    }

    repair_persisted_seed_json_noise(conn)?;

    Ok(())
}

fn repair_persisted_seed_json_noise(conn: &word_storage_core::Connection) -> Result<(), String> {
    let keys = [
        "ai_passage_history_json",
        "active_study_session_newWord",
        "active_study_session_review",
        "active_study_session_mixedTest",
        "active_study_session_wrongWordReinforcement",
        "active_study_session_rootAffix",
    ];
    for key in keys {
        let raw = conn
            .query_row(
                "SELECT value_json FROM app_settings WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("Failed to load persisted seed JSON noise candidate: {e}"))?;
        let Some(raw) = raw else {
            continue;
        };
        let Ok(mut value) = serde_json::from_str::<serde_json::Value>(&raw) else {
            continue;
        };
        clean_seed_meaning_noise_in_json(&mut value);
        let cleaned = value.to_string();
        if cleaned != raw {
            set_json_setting(conn, key, &value)?;
        }
    }
    Ok(())
}

fn ensure_planning_state(conn: &word_storage_core::Connection) -> Result<(), String> {
    ensure_setting(conn, "saved_plan_json", &default_plan_json())?;
    ensure_setting(conn, "today_plan_json", &default_plan_json())?;
    ensure_setting(conn, "today_snapshot_seed_json", &serde_json::json!({}))?;
    ensure_setting(
        conn,
        "saved_wordbooks_json",
        &default_wordbook_selection_json(),
    )?;
    ensure_setting(
        conn,
        "today_wordbooks_json",
        &default_wordbook_selection_json(),
    )?;
    Ok(())
}

fn ensure_ai_provider_config(conn: &word_storage_core::Connection) -> Result<(), String> {
    let default_value = serde_json::to_value(default_ai_provider_config())
        .map_err(|e| format!("Failed to serialize default AI provider config: {e}"))?;
    ensure_setting(conn, "ai_provider_config_json", &default_value)
}

fn ensure_setting(
    conn: &word_storage_core::Connection,
    key: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)
         ON CONFLICT(key) DO NOTHING",
        rusqlite::params![key, value.to_string()],
    )
    .map_err(|e| format!("Failed to seed setting {key}: {e}"))?;
    Ok(())
}

fn get_json_setting(
    conn: &word_storage_core::Connection,
    key: &str,
    default_value: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT value_json FROM app_settings WHERE key = ?1",
            rusqlite::params![key],
            |row| row.get(0),
        )
        .ok();
    match raw {
        Some(value) => {
            serde_json::from_str(&value).map_err(|e| format!("Invalid JSON for {key}: {e}"))
        }
        None => Ok(default_value.clone()),
    }
}

fn set_json_setting(
    conn: &word_storage_core::Connection,
    key: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_settings (key, value_json, updated_at) VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at",
        rusqlite::params![key, value.to_string()],
    )
    .map_err(|e| format!("Failed to store setting {key}: {e}"))?;
    Ok(())
}

fn load_today_target_seed(
    conn: &word_storage_core::Connection,
    today_date: &str,
) -> Result<Option<TodayTargetSeed>, String> {
    let value = get_json_setting(conn, "today_snapshot_seed_json", &serde_json::json!({}))?;
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    let stored_date = object.get("date").and_then(|value| value.as_str());
    if stored_date != Some(today_date) {
        return Ok(None);
    }
    let Some(targets_value) = object.get("targets") else {
        return Ok(None);
    };
    let targets: TodayTargetSeed = serde_json::from_value(targets_value.clone())
        .map_err(|e| format!("Invalid today snapshot seed: {e}"))?;
    Ok(Some(targets))
}

fn persist_today_target_seed(
    conn: &word_storage_core::Connection,
    today_date: &str,
    plan_value: &serde_json::Value,
) -> Result<TodayTargetSeed, String> {
    let targets = today_target_seed_from_plan_value(plan_value);
    let targets_value = serde_json::to_value(&targets)
        .map_err(|e| format!("Failed to serialize today snapshot seed: {e}"))?;
    let payload = serde_json::json!({
        "date": today_date,
        "targets": targets_value,
    });
    set_json_setting(conn, "today_snapshot_seed_json", &payload)?;
    Ok(targets)
}

fn load_today_reward_state(
    conn: &word_storage_core::Connection,
    today_date: &str,
) -> Result<serde_json::Value, String> {
    let value = get_json_setting(conn, "today_reward_state_json", &serde_json::json!({}))?;
    let Some(object) = value.as_object() else {
        return Ok(serde_json::json!({ "todayDate": today_date }));
    };
    let stored_date = object.get("todayDate").and_then(|value| value.as_str());
    if stored_date != Some(today_date) {
        return Ok(serde_json::json!({ "todayDate": today_date }));
    }
    Ok(value)
}

fn today_target_seed_from_plan_value(plan_value: &serde_json::Value) -> TodayTargetSeed {
    let today = today_date_string();
    today_target_seed_from_plan_value_for_date(plan_value, &today)
}

fn today_target_seed_from_plan_value_for_date(
    plan_value: &serde_json::Value,
    today_date: &str,
) -> TodayTargetSeed {
    let new_word_units = grown_plan_unit_count(plan_value, "newWord", "newWordsPerDay", today_date);
    let review_units = grown_plan_unit_count(plan_value, "review", "reviewWordsPerDay", today_date);
    let mixed_test = grown_plan_unit_count(plan_value, "mixedTest", "mixedTestPerDay", today_date);
    let wrong_word_test = grown_plan_unit_count(
        plan_value,
        "wrongWordReinforcement",
        "wrongWordTestPerDay",
        today_date,
    );
    let root_affix = grown_plan_unit_count(plan_value, "rootAffix", "rootAffixPerDay", today_date);

    let new_words = new_word_units.saturating_mul(4);
    let review_words = review_units.saturating_mul(4);

    TodayTargetSeed {
        new_words_target: Some(new_words),
        new_words_base_target: Some(new_words),
        new_words_carryover_target: Some(0),
        review_words_target: Some(review_words),
        review_words_base_target: Some(review_words),
        review_words_carryover_target: Some(0),
        mixed_test_target: Some(mixed_test),
        mixed_test_base_target: Some(mixed_test),
        mixed_test_carryover_target: Some(0),
        wrong_word_test_target: Some(wrong_word_test),
        wrong_word_test_base_target: Some(wrong_word_test),
        wrong_word_test_carryover_target: Some(0),
        root_affix_target: Some(root_affix),
        root_affix_base_target: Some(root_affix),
        root_affix_carryover_target: Some(0),
    }
}

fn align_today_targets_to_available_pools(
    conn: &word_storage_core::Connection,
    mut targets: TodayTargetSeed,
    plan_value: &serde_json::Value,
) -> Result<TodayTargetSeed, String> {
    let active_wordbook_ids = selected_wordbook_ids_for_today(conn)?;
    let review_wordbook_ids = selected_review_wordbook_ids_for_today(conn)?;
    let today = today_date_string();
    let new_word_units =
        grown_plan_unit_count(plan_value, "newWord", "newWordsPerDay", &today) as usize;
    let review_units =
        grown_plan_unit_count(plan_value, "review", "reviewWordsPerDay", &today) as usize;
    let mixed_units =
        grown_plan_unit_count(plan_value, "mixedTest", "mixedTestPerDay", &today) as usize;
    let wrong_word_units = grown_plan_unit_count(
        plan_value,
        "wrongWordReinforcement",
        "wrongWordTestPerDay",
        &today,
    ) as usize;
    let root_affix_units =
        grown_plan_unit_count(plan_value, "rootAffix", "rootAffixPerDay", &today) as usize;

    let available_new_words = load_unlearned_ranked_entry_ids_for_wordbooks_with_global_fallback(
        conn,
        &active_wordbook_ids,
        new_word_units,
    )?
    .len() as u32;
    let available_review_words =
        load_review_entry_ids_for_today(conn, &review_wordbook_ids, review_units)?.len() as u32;
    let available_mixed_tests = load_ranked_entry_ids_for_wordbooks_with_global_fallback(
        conn,
        &active_wordbook_ids,
        mixed_units,
    )?
    .len() as u32;
    let available_wrong_words =
        load_prioritized_wrong_word_entry_ids(conn, wrong_word_units)?.len() as u32;

    let new_target = available_new_words.saturating_mul(4);
    let review_target = available_review_words.saturating_mul(4);
    let mixed_target = available_mixed_tests;
    let wrong_target = available_wrong_words;
    let root_target = root_affix_units as u32;

    targets.new_words_target = Some(new_target);
    targets.new_words_base_target = Some(new_target);
    targets.review_words_target = Some(review_target);
    targets.review_words_base_target = Some(review_target);
    targets.mixed_test_target = Some(mixed_target);
    targets.mixed_test_base_target = Some(mixed_target);
    targets.wrong_word_test_target = Some(wrong_target);
    targets.wrong_word_test_base_target = Some(wrong_target);
    targets.root_affix_target = Some(root_target);
    targets.root_affix_base_target = Some(root_target);

    Ok(targets)
}

fn plan_unit_count(plan_value: &serde_json::Value, key: &str) -> usize {
    plan_value
        .get(key)
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0) as usize
}

fn grown_plan_unit_count(
    plan_value: &serde_json::Value,
    mode: &str,
    plan_key: &str,
    today_date: &str,
) -> u32 {
    let base = plan_unit_count(plan_value, plan_key) as u32;
    let rule = growth_rule_for_mode(plan_value, mode);
    let interval_days = rule
        .get("intervalDays")
        .and_then(|value| value.as_i64())
        .unwrap_or(7)
        .max(1);
    let increment = rule
        .get("increment")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0) as u32;
    if base == 0 || increment == 0 {
        return base;
    }

    let start_date = growth_start_date_for_mode(plan_value, mode, today_date);
    let elapsed_days = days_between_dates(&start_date, today_date).unwrap_or(0);
    let growth_steps = (elapsed_days / interval_days).max(0) as u32;
    base.saturating_add(growth_steps.saturating_mul(increment))
}

fn growth_rule_for_mode(plan_value: &serde_json::Value, mode: &str) -> serde_json::Value {
    if plan_value
        .get("growthRuleMode")
        .and_then(|value| value.as_str())
        .unwrap_or("shared")
        == "perMode"
    {
        if let Some(rule) = plan_value
            .get("growthRulesByMode")
            .and_then(|value| value.as_object())
            .and_then(|rules| rules.get(mode))
        {
            return rule.clone();
        }
    }
    growth_rule_from_value(plan_value)
}

fn growth_start_date_for_mode(
    plan_value: &serde_json::Value,
    mode: &str,
    fallback_date: &str,
) -> String {
    if plan_value
        .get("growthRuleMode")
        .and_then(|value| value.as_str())
        .unwrap_or("shared")
        == "perMode"
    {
        if let Some(start_date) = plan_value
            .get("growthRuleStartDatesByMode")
            .and_then(|value| value.as_object())
            .and_then(|starts| starts.get(mode))
            .and_then(|value| value.as_str())
        {
            return start_date.to_string();
        }
    }
    plan_value
        .get("growthRuleStartDate")
        .and_then(|value| value.as_str())
        .unwrap_or(fallback_date)
        .to_string()
}

fn days_between_dates(start_date: &str, end_date: &str) -> Option<i64> {
    let start = chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d").ok()?;
    let end = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d").ok()?;
    Some((end - start).num_days())
}

fn merge_json_object(target: &mut serde_json::Value, input: &serde_json::Value) {
    let Some(target_obj) = target.as_object_mut() else {
        return;
    };
    let Some(input_obj) = input.as_object() else {
        return;
    };
    for (key, value) in input_obj {
        target_obj.insert(key.clone(), value.clone());
    }
}

fn today_date_string() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn growth_rule_from_value(value: &serde_json::Value) -> serde_json::Value {
    if let Some(rule) = value.get("sharedGrowthRule") {
        return rule.clone();
    }
    serde_json::json!({
        "intervalDays": value.get("growthIntervalDays").and_then(|value| value.as_i64()).unwrap_or(7),
        "increment": value.get("growthIncrement").and_then(|value| value.as_i64()).unwrap_or(5)
    })
}

fn same_growth_rule(left: &serde_json::Value, right: &serde_json::Value) -> bool {
    left.get("intervalDays")
        .and_then(|value| value.as_i64())
        .unwrap_or(7)
        == right
            .get("intervalDays")
            .and_then(|value| value.as_i64())
            .unwrap_or(7)
        && left
            .get("increment")
            .and_then(|value| value.as_i64())
            .unwrap_or(5)
            == right
                .get("increment")
                .and_then(|value| value.as_i64())
                .unwrap_or(5)
}

fn growth_rules_by_mode_from_value(
    value: &serde_json::Value,
) -> serde_json::Map<String, serde_json::Value> {
    let shared = growth_rule_from_value(value);
    let existing = value
        .get("growthRulesByMode")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let mut out = serde_json::Map::new();
    for mode in [
        "newWord",
        "review",
        "mixedTest",
        "wrongWordReinforcement",
        "rootAffix",
    ] {
        out.insert(
            mode.to_string(),
            existing
                .get(mode)
                .cloned()
                .unwrap_or_else(|| shared.clone()),
        );
    }
    out
}

fn stamp_growth_rule_start_dates(
    previous_plan: &serde_json::Value,
    input: &serde_json::Value,
    next_plan: &mut serde_json::Value,
) {
    let next_snapshot = next_plan.clone();
    let previous_shared = growth_rule_from_value(previous_plan);
    let next_shared_fallback = growth_rule_from_value(&next_snapshot);
    let previous_rules = growth_rules_by_mode_from_value(previous_plan);
    let next_rules = growth_rules_by_mode_from_value(&next_snapshot);
    let Some(next_obj) = next_plan.as_object_mut() else {
        return;
    };
    let today = today_date_string();
    let previous_mode = previous_plan
        .get("growthRuleMode")
        .and_then(|value| value.as_str())
        .unwrap_or("shared");
    let next_mode = next_obj
        .get("growthRuleMode")
        .and_then(|value| value.as_str())
        .unwrap_or(previous_mode);
    let mode_changed = next_mode != previous_mode;

    let next_shared = next_obj
        .get("sharedGrowthRule")
        .cloned()
        .unwrap_or_else(|| next_shared_fallback.clone());
    next_obj.insert("sharedGrowthRule".to_string(), next_shared.clone());
    if mode_changed || !same_growth_rule(&previous_shared, &next_shared) {
        next_obj.insert(
            "growthRuleStartDate".to_string(),
            serde_json::Value::String(today.clone()),
        );
    } else if !next_obj.contains_key("growthRuleStartDate") {
        next_obj.insert(
            "growthRuleStartDate".to_string(),
            serde_json::Value::String(
                previous_plan
                    .get("growthRuleStartDate")
                    .and_then(|value| value.as_str())
                    .unwrap_or(&today)
                    .to_string(),
            ),
        );
    }

    next_obj.insert(
        "growthRulesByMode".to_string(),
        serde_json::Value::Object(next_rules.clone()),
    );
    let previous_starts = previous_plan
        .get("growthRuleStartDatesByMode")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let mut next_starts = serde_json::Map::new();
    for mode in [
        "newWord",
        "review",
        "mixedTest",
        "wrongWordReinforcement",
        "rootAffix",
    ] {
        let previous_rule = previous_rules
            .get(mode)
            .cloned()
            .unwrap_or_else(|| previous_shared.clone());
        let next_rule = next_rules
            .get(mode)
            .cloned()
            .unwrap_or_else(|| next_shared.clone());
        let start = if mode_changed || !same_growth_rule(&previous_rule, &next_rule) {
            today.clone()
        } else {
            previous_starts
                .get(mode)
                .and_then(|value| value.as_str())
                .unwrap_or(&today)
                .to_string()
        };
        next_starts.insert(mode.to_string(), serde_json::Value::String(start));
    }
    next_obj.insert(
        "growthRuleStartDatesByMode".to_string(),
        serde_json::Value::Object(next_starts),
    );

    if input.get("growthIntervalDays").is_some() && input.get("sharedGrowthRule").is_none() {
        next_obj.insert(
            "sharedGrowthRule".to_string(),
            serde_json::json!({
                "intervalDays": next_obj.get("growthIntervalDays").and_then(|value| value.as_i64()).unwrap_or(7),
                "increment": next_obj.get("growthIncrement").and_then(|value| value.as_i64()).unwrap_or(5)
            }),
        );
    }
}

fn default_plan_json() -> serde_json::Value {
    let today = today_date_string();
    let shared_growth_rule = serde_json::json!({
        "intervalDays": 7,
        "increment": 5
    });
    serde_json::json!({
        "id": 1,
        "name": "Starter Plan",
        "newWordsPerDay": 5,
        "reviewWordsPerDay": 6,
        "mixedTestPerDay": 4,
        "wrongWordTestPerDay": 3,
        "rootAffixPerDay": 2,
        "growthIntervalDays": 7,
        "growthIncrement": 5,
        "growthRuleMode": "shared",
        "growthRuleStartDate": today,
        "sharedGrowthRule": shared_growth_rule.clone(),
        "growthRulesByMode": {
            "newWord": shared_growth_rule.clone(),
            "review": shared_growth_rule.clone(),
            "mixedTest": shared_growth_rule.clone(),
            "wrongWordReinforcement": shared_growth_rule.clone(),
            "rootAffix": shared_growth_rule.clone()
        },
        "growthRuleStartDatesByMode": {
            "newWord": today,
            "review": today,
            "mixedTest": today,
            "wrongWordReinforcement": today,
            "rootAffix": today
        }
    })
}

fn default_wordbook_selection_json() -> serde_json::Value {
    serde_json::json!({
        "1": true,
        "2": false,
        "3": false,
        "4": false
    })
}

fn assemble_wordbooks(selection: &serde_json::Value) -> Vec<serde_json::Value> {
    let lookup = selection.as_object().cloned().unwrap_or_default();
    [
        (1i64, "cet4", "CET-4", "exam", 4500i64),
        (2i64, "cet6", "CET-6", "exam", 3000i64),
        (3i64, "kaoyan", "KaoYan", "exam", 5500i64),
        (4i64, "medical", "Medical English", "specialized", 1800i64),
    ]
    .into_iter()
    .map(|(id, code, name, category, total_entries)| {
        serde_json::json!({
            "id": id,
            "code": code,
            "name": name,
            "category": category,
            "totalEntries": total_entries,
            "isActive": lookup.get(&id.to_string()).and_then(|value| value.as_bool()).unwrap_or(false)
        })
    })
    .collect()
}

// ============================================================================
// Settings API
// ============================================================================

/// Get settings summary.
///
/// Returns a JSON string containing SettingsSummary.
pub fn get_settings() -> Result<String, String> {
    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;
    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;
    ensure_planning_state(&conn)?;
    ensure_ai_provider_config(&conn)?;
    let resolved = resolve_ai_provider_config();
    let settings = serde_json::json!({
        "aiConfigured": !resolved.primary.auth_token.trim().is_empty() && !resolved.backup.auth_token.trim().is_empty(),
        "syncConfigured": false,
        "appVersion": "0.1.0",
        "schemaVersion": 7,
        "databasePath": db_path.display().to_string()
    });

    serde_json::to_string(&settings).map_err(|e| format!("JSON serialization failed: {}", e))
}

pub fn get_sync_status() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let mut status = serde_json::to_value(
            persistence::sync_repo::get_sync_status(conn)
                .map_err(|e| format!("Failed to get sync status: {e}"))?,
        )
        .map_err(|e| format!("JSON serialization failed: {}", e))?;
        if let Some(status_obj) = status.as_object_mut() {
            let last_restore = get_json_setting(
                conn,
                "cloud_restore_last_result_json",
                &serde_json::Value::Null,
            )?;
            status_obj.insert("lastCloudRestore".to_string(), last_restore);
        }
        serde_json::to_string(&status).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

fn set_cloud_restore_result(
    conn: &word_storage_core::Connection,
    value: serde_json::Value,
) -> Result<(), String> {
    set_json_setting(conn, "cloud_restore_last_result_json", &value)
}

pub fn record_cloud_restore_attempt(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    with_runtime_conn(|conn| {
        let value = serde_json::json!({
            "attemptedAt": chrono::Local::now().to_rfc3339(),
            "userId": request.get("userId").and_then(|value| value.as_str()).unwrap_or(""),
            "succeeded": request.get("succeeded").and_then(|value| value.as_bool()).unwrap_or(false),
            "planRows": request.get("planRows").and_then(json_i64_value).unwrap_or(0),
            "wordbookRows": request.get("wordbookRows").and_then(json_i64_value).unwrap_or(0),
            "studyPointRows": request.get("studyPointRows").and_then(json_i64_value).unwrap_or(0),
            "reportSnapshotRows": request.get("reportSnapshotRows").and_then(json_i64_value).unwrap_or(0),
            "restoredStudyPoints": request.get("restoredStudyPoints").and_then(json_i64_value).unwrap_or(0),
            "restoredReportSnapshots": request.get("restoredReportSnapshots").and_then(json_i64_value).unwrap_or(0),
            "error": request.get("error").and_then(|value| value.as_str()),
        });
        set_cloud_restore_result(conn, value)?;
        Ok(serde_json::json!({"recorded": true}).to_string())
    })
}

pub fn enqueue_cloud_backfill(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    let window_days = request
        .get("windowDays")
        .and_then(json_i64_value)
        .unwrap_or(365)
        .clamp(1, 3650);
    with_runtime_conn(|conn| {
        let study_points = match build_recent_study_word_points_payload(conn, window_days)? {
            Some(payload) => {
                let point_count = payload
                    .get("points")
                    .and_then(|value| value.as_array())
                    .map(|items| items.len())
                    .unwrap_or(0);
                enqueue_sync_snapshot(
                    conn,
                    "study_word_points",
                    &payload,
                    &format!("study_word_points:backfill_{window_days}_days"),
                );
                point_count
            }
            None => 0,
        };
        let wrong_words = match build_wrong_word_entries_payload(conn)? {
            Some(payload) => {
                let wrong_word_count = payload
                    .get("entries")
                    .and_then(|value| value.as_array())
                    .map(|items| items.len())
                    .unwrap_or(0);
                enqueue_sync_snapshot(
                    conn,
                    "wrong_word_entries",
                    &payload,
                    "wrong_word_entries:backfill_current_snapshot",
                );
                wrong_word_count
            }
            None => 0,
        };
        let ai_passages = enqueue_ai_passages_snapshot(conn)?;
        Ok(serde_json::json!({
            "enqueued": study_points > 0 || wrong_words > 0 || ai_passages > 0,
            "studyPoints": study_points,
            "wrongWords": wrong_words,
            "aiPassages": ai_passages,
            "windowDays": window_days,
        })
        .to_string())
    })
}

pub fn get_sync_status_legacy() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let status = persistence::sync_repo::get_sync_status(conn)
            .map_err(|e| format!("Failed to get sync status: {e}"))?;
        serde_json::to_string(&status).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn record_sync_result(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    let item_id = request
        .get("itemId")
        .and_then(|value| value.as_i64())
        .ok_or("recordSyncResult requires itemId")?;
    let succeeded = request
        .get("succeeded")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    with_runtime_conn(|conn| {
        if succeeded {
            persistence::sync_repo::mark_outbox_item_succeeded(conn, item_id)
                .map_err(|e| format!("Failed to mark sync success: {e}"))?;
        } else {
            let failure_code = request
                .get("failureCode")
                .and_then(|value| value.as_str())
                .unwrap_or("upload_failed");
            let failure_message = request
                .get("failureMessage")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            persistence::sync_repo::mark_outbox_item_retryable_failure(
                conn,
                item_id,
                failure_code,
                failure_message,
            )
            .map_err(|e| format!("Failed to mark sync failure: {e}"))?;
        }
        Ok(serde_json::json!({"recorded": true}).to_string())
    })
}

pub fn reconcile_local_data_owner(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    let user_id = request
        .get("userId")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("userId is required")?;

    with_runtime_conn(|conn| {
        let current_owner =
            get_json_setting(conn, "cloud_data_owner_user_id", &serde_json::Value::Null)?
                .as_str()
                .map(str::to_string);

        let reset_performed = current_owner
            .as_deref()
            .is_some_and(|owner| owner != user_id);
        let mut restored_snapshot = false;
        if reset_performed {
            if let Some(owner) = current_owner.as_deref() {
                save_local_account_snapshot(conn, owner)?;
            }
            restored_snapshot = restore_local_account_snapshot(conn, user_id)?;
        }
        let has_local_learning_data = has_local_user_learning_data(conn)?;

        set_json_setting(
            conn,
            "cloud_data_owner_user_id",
            &serde_json::Value::String(user_id.to_string()),
        )?;
        serde_json::to_string(&serde_json::json!({
            "ownerUserId": user_id,
            "previousOwnerUserId": current_owner,
            "resetPerformed": reset_performed,
            "restoredSnapshot": restored_snapshot,
            "hasLocalLearningData": has_local_learning_data,
        }))
        .map_err(|e| format!("JSON serialization failed: {e}"))
    })
}

pub fn switch_to_guest_local_data() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let current_owner =
            get_json_setting(conn, "cloud_data_owner_user_id", &serde_json::Value::Null)?
                .as_str()
                .map(str::to_string);
        if let Some(owner) = current_owner.as_deref() {
            if owner != "guest" {
                save_local_account_snapshot(conn, owner)?;
            } else if has_local_user_learning_data(conn)? {
                save_local_account_snapshot(conn, "guest")?;
            }
        } else if has_local_user_learning_data(conn)? {
            save_local_account_snapshot(conn, "guest")?;
        }
        let restored_guest = restore_local_account_snapshot(conn, "guest")?;
        set_json_setting(
            conn,
            "cloud_data_owner_user_id",
            &serde_json::Value::String("guest".to_string()),
        )?;
        serde_json::to_string(&serde_json::json!({
            "ownerUserId": "guest",
            "previousOwnerUserId": current_owner,
            "restoredGuestSnapshot": restored_guest,
        }))
        .map_err(|e| format!("JSON serialization failed: {e}"))
    })
}

const USER_OWNED_SETTING_KEYS: &[&str] = &[
    "saved_plan_json",
    "today_plan_json",
    "today_snapshot_seed_json",
    "saved_wordbooks_json",
    "today_wordbooks_json",
    "today_review_wordbooks_json",
    "today_reward_state_json",
    "ai_passage_history_json",
    "cloud_report_snapshots_json",
];

fn local_account_snapshot_key(user_id: &str) -> String {
    format!("local_account_snapshot:{user_id}")
}

fn save_local_account_snapshot(
    conn: &word_storage_core::Connection,
    user_id: &str,
) -> Result<(), String> {
    let snapshot = serde_json::json!({
        "version": 1,
        "savedAt": chrono::Local::now().to_rfc3339(),
        "studySessions": export_study_sessions(conn)?,
        "studyResults": export_study_results(conn)?,
        "importedWrongWords": export_imported_wrong_words(conn)?,
        "wordHints": export_word_hints(conn)?,
        "settings": export_user_owned_settings(conn)?,
    });
    set_json_setting(conn, &local_account_snapshot_key(user_id), &snapshot)
}

fn has_local_user_learning_data(conn: &word_storage_core::Connection) -> Result<bool, String> {
    let count: i64 = conn
        .query_row(
            "SELECT
                (SELECT COUNT(*) FROM study_results) +
                (SELECT COUNT(*) FROM study_sessions) +
                (SELECT COUNT(*) FROM imported_wrong_words) +
                (SELECT COUNT(*) FROM word_hints)",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to inspect local learning data: {e}"))?;
    Ok(count > 0)
}

fn restore_local_account_snapshot(
    conn: &word_storage_core::Connection,
    user_id: &str,
) -> Result<bool, String> {
    let snapshot = get_json_setting(
        conn,
        &local_account_snapshot_key(user_id),
        &serde_json::Value::Null,
    )?;
    reset_user_owned_local_data(conn)?;
    if snapshot.is_null() {
        return Ok(false);
    }
    import_user_owned_settings(conn, snapshot.get("settings"))?;
    import_study_sessions(conn, snapshot.get("studySessions"))?;
    import_study_results(conn, snapshot.get("studyResults"))?;
    import_imported_wrong_words(conn, snapshot.get("importedWrongWords"))?;
    import_word_hints(conn, snapshot.get("wordHints"))?;
    Ok(true)
}

fn reset_user_owned_local_data(conn: &word_storage_core::Connection) -> Result<(), String> {
    conn.execute_batch(
        "DELETE FROM study_results;
         DELETE FROM study_sessions;
         DELETE FROM imported_wrong_words;
         DELETE FROM word_hints;
         DELETE FROM sync_outbox;
         DELETE FROM sync_dead_letter;
         DELETE FROM sync_cursor_state;",
    )
    .map_err(|e| format!("Failed to reset user-owned local tables: {e}"))?;

    conn.execute(
        "DELETE FROM app_settings
         WHERE key IN (
            'saved_plan_json',
            'today_plan_json',
            'today_snapshot_seed_json',
            'saved_wordbooks_json',
            'today_wordbooks_json',
             'today_review_wordbooks_json',
             'today_reward_state_json',
            'ai_passage_history_json',
            'cloud_report_snapshots_json'
         )
         OR key LIKE 'active_study_session_%'",
        [],
    )
    .map_err(|e| format!("Failed to reset user-owned local settings: {e}"))?;
    core_clear_all_active_sessions();
    ensure_planning_state(conn)?;
    Ok(())
}

fn export_user_owned_settings(
    conn: &word_storage_core::Connection,
) -> Result<Vec<serde_json::Value>, String> {
    let mut settings = Vec::new();
    for key in USER_OWNED_SETTING_KEYS {
        if let Some(value_json) = conn
            .query_row(
                "SELECT value_json FROM app_settings WHERE key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| format!("Failed to export setting {key}: {e}"))?
        {
            settings.push(serde_json::json!({
                "key": key,
                "valueJson": value_json,
            }));
        }
    }
    let mut stmt = conn
        .prepare("SELECT key, value_json FROM app_settings WHERE key LIKE 'active_study_session_%'")
        .map_err(|e| format!("Failed to prepare active session setting export: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "key": row.get::<_, String>(0)?,
                "valueJson": row.get::<_, String>(1)?,
            }))
        })
        .map_err(|e| format!("Failed to export active session settings: {e}"))?;
    for row in rows {
        settings.push(row.map_err(|e| format!("Failed to decode active session setting: {e}"))?);
    }
    Ok(settings)
}

fn import_user_owned_settings(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<(), String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(());
    };
    for item in items {
        let Some(key) = item.get("key").and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(value_json) = item.get("valueJson").and_then(|value| value.as_str()) else {
            continue;
        };
        conn.execute(
            "INSERT INTO app_settings (key, value_json, updated_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at = excluded.updated_at",
            rusqlite::params![key, value_json],
        )
        .map_err(|e| format!("Failed to restore setting {key}: {e}"))?;
    }
    Ok(())
}

fn export_study_sessions(
    conn: &word_storage_core::Connection,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, mode, total_words, wordbook_id, started_at, completed_at
             FROM study_sessions ORDER BY id ASC",
        )
        .map_err(|e| format!("Failed to prepare study session export: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "sessionId": row.get::<_, String>(1)?,
                "mode": row.get::<_, String>(2)?,
                "totalWords": row.get::<_, i64>(3)?,
                "wordbookId": row.get::<_, Option<i64>>(4)?,
                "startedAt": row.get::<_, String>(5)?,
                "completedAt": row.get::<_, Option<String>>(6)?,
            }))
        })
        .map_err(|e| format!("Failed to export study sessions: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode study sessions: {e}"))
}

fn import_study_sessions(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<(), String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(());
    };
    for item in items {
        conn.execute(
            "INSERT OR REPLACE INTO study_sessions
             (id, session_id, mode, total_words, wordbook_id, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                item.get("id").and_then(|value| value.as_i64()),
                item.get("sessionId")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("mode")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("totalWords")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0),
                item.get("wordbookId").and_then(|value| value.as_i64()),
                item.get("startedAt")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("completedAt").and_then(|value| value.as_str()),
            ],
        )
        .map_err(|e| format!("Failed to restore study session: {e}"))?;
    }
    Ok(())
}

fn export_study_results(
    conn: &word_storage_core::Connection,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, question_id, entry_id, question_type, user_response,
                    normalized_response, correct_answer, outcome, response_time_ms, answered_at
             FROM study_results ORDER BY id ASC",
        )
        .map_err(|e| format!("Failed to prepare study result export: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "sessionId": row.get::<_, String>(1)?,
                "questionId": row.get::<_, String>(2)?,
                "entryId": row.get::<_, i64>(3)?,
                "questionType": row.get::<_, String>(4)?,
                "userResponse": row.get::<_, String>(5)?,
                "normalizedResponse": row.get::<_, Option<String>>(6)?,
                "correctAnswer": row.get::<_, String>(7)?,
                "outcome": row.get::<_, String>(8)?,
                "responseTimeMs": row.get::<_, i64>(9)?,
                "answeredAt": row.get::<_, String>(10)?,
            }))
        })
        .map_err(|e| format!("Failed to export study results: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode study results: {e}"))
}

fn import_study_results(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<(), String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(());
    };
    for item in items {
        conn.execute(
            "INSERT OR REPLACE INTO study_results
             (id, session_id, question_id, entry_id, question_type, user_response,
              normalized_response, correct_answer, outcome, response_time_ms, answered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            rusqlite::params![
                item.get("id").and_then(|value| value.as_i64()),
                item.get("sessionId")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("questionId")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("entryId")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0),
                item.get("questionType")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("userResponse")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("normalizedResponse")
                    .and_then(|value| value.as_str()),
                item.get("correctAnswer")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("outcome")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("responseTimeMs")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0),
                item.get("answeredAt")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
            ],
        )
        .map_err(|e| format!("Failed to restore study result: {e}"))?;
    }
    Ok(())
}

fn export_imported_wrong_words(
    conn: &word_storage_core::Connection,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, batch_id, candidate_id, source_type, source_name, entry_id,
                    word, meaning, occurrence_count, confidence, evidence,
                    is_high_frequency, imported_at
             FROM imported_wrong_words ORDER BY id ASC",
        )
        .map_err(|e| format!("Failed to prepare imported wrong word export: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i64>(0)?,
                "batchId": row.get::<_, String>(1)?,
                "candidateId": row.get::<_, String>(2)?,
                "sourceType": row.get::<_, String>(3)?,
                "sourceName": row.get::<_, String>(4)?,
                "entryId": row.get::<_, Option<i64>>(5)?,
                "word": row.get::<_, String>(6)?,
                "meaning": row.get::<_, Option<String>>(7)?,
                "occurrenceCount": row.get::<_, i64>(8)?,
                "confidence": row.get::<_, f64>(9)?,
                "evidence": row.get::<_, String>(10)?,
                "isHighFrequency": row.get::<_, i64>(11)?,
                "importedAt": row.get::<_, String>(12)?,
            }))
        })
        .map_err(|e| format!("Failed to export imported wrong words: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode imported wrong words: {e}"))
}

fn import_imported_wrong_words(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<(), String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(());
    };
    for item in items {
        conn.execute(
            "INSERT OR REPLACE INTO imported_wrong_words
             (id, batch_id, candidate_id, source_type, source_name, entry_id,
              word, meaning, occurrence_count, confidence, evidence,
              is_high_frequency, imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                item.get("id").and_then(|value| value.as_i64()),
                item.get("batchId")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("candidateId")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("sourceType")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("sourceName")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("entryId").and_then(|value| value.as_i64()),
                item.get("word")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("meaning").and_then(|value| value.as_str()),
                item.get("occurrenceCount")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(1),
                item.get("confidence")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.0),
                item.get("evidence")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
                item.get("isHighFrequency")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0),
                item.get("importedAt")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
            ],
        )
        .map_err(|e| format!("Failed to restore imported wrong word: {e}"))?;
    }
    Ok(())
}

fn export_word_hints(
    conn: &word_storage_core::Connection,
) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT entry_id, hint_text, source, updated_at
             FROM word_hints
             WHERE TRIM(hint_text) <> ''
             ORDER BY entry_id ASC",
        )
        .map_err(|e| format!("Failed to prepare word hint export: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "entryId": row.get::<_, i64>(0)?,
                "hintText": row.get::<_, String>(1)?,
                "hintSource": row.get::<_, String>(2)?,
                "hintUpdatedAt": row.get::<_, String>(3)?,
            }))
        })
        .map_err(|e| format!("Failed to export word hints: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode word hints: {e}"))
}

fn import_word_hints(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<(), String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(());
    };
    for item in items {
        let Some(entry_id) = item.get("entryId").and_then(json_i64_value) else {
            continue;
        };
        let hint_text = item
            .get("hintText")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let hint_source = item
            .get("hintSource")
            .and_then(|value| value.as_str())
            .unwrap_or("user");
        persistence::word_hint_repo::save_hint(conn, entry_id, hint_text, hint_source)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn restore_cloud_data_snapshot(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    let user_id = request
        .get("userId")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("userId is required")?;

    with_runtime(|runtime, conn| {
        reset_user_owned_local_data(conn)?;
        ensure_seed_vocabulary_imported(conn, &runtime.paths().bundled_resource_path(""))?;
        let restored_plan = restore_cloud_plan_config(conn, request.get("planConfig"))?;
        let restored_wordbooks =
            restore_cloud_wordbook_preferences(conn, request.get("wordbookPreferences"))?;
        let restored_points =
            restore_cloud_study_word_points(conn, request.get("studyWordPoints"))?;
        let restored_report_snapshots =
            restore_cloud_report_snapshots(conn, request.get("reportSnapshots"))?;
        let restored_word_hints =
            restore_cloud_wrong_word_hints(conn, request.get("wrongWordEntries"))?;
        ensure_planning_state(conn)?;
        save_local_account_snapshot(conn, user_id)?;
        serde_json::to_string(&serde_json::json!({
            "restored": true,
            "restoredPlan": restored_plan,
            "restoredWordbooks": restored_wordbooks,
            "restoredStudyPoints": restored_points,
            "restoredReportSnapshots": restored_report_snapshots,
            "restoredWordHints": restored_word_hints,
        }))
        .map_err(|e| format!("JSON serialization failed: {e}"))
    })
}

fn restore_cloud_plan_config(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<bool, String> {
    let Some(plan_row) = value.and_then(|value| value.as_object()) else {
        return Ok(false);
    };
    let mut plan = default_plan_json();
    let Some(plan_obj) = plan.as_object_mut() else {
        return Ok(false);
    };

    set_json_field(plan_obj, "name", plan_row.get("name"));
    set_i64_json_field(
        plan_obj,
        "newWordsPerDay",
        plan_row.get("new_words_per_day"),
    );
    set_i64_json_field(
        plan_obj,
        "reviewWordsPerDay",
        plan_row.get("review_words_per_day"),
    );
    set_i64_json_field(
        plan_obj,
        "mixedTestPerDay",
        plan_row.get("mixed_test_per_day"),
    );
    set_i64_json_field(
        plan_obj,
        "wrongWordTestPerDay",
        plan_row.get("wrong_word_test_per_day"),
    );
    set_i64_json_field(
        plan_obj,
        "rootAffixPerDay",
        plan_row.get("root_affix_per_day"),
    );
    set_json_field(plan_obj, "growthRuleMode", plan_row.get("growth_rule_mode"));
    set_json_field(
        plan_obj,
        "sharedGrowthRule",
        plan_row.get("shared_growth_rule"),
    );
    set_json_field(
        plan_obj,
        "growthRulesByMode",
        plan_row.get("growth_rules_by_mode"),
    );
    set_json_setting(conn, "saved_plan_json", &plan)?;
    set_json_setting(conn, "today_plan_json", &plan)?;
    Ok(true)
}

fn set_json_field(
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<&serde_json::Value>,
) {
    if let Some(value) = value.filter(|value| !value.is_null()) {
        target.insert(key.to_string(), value.clone());
    }
}

fn set_i64_json_field(
    target: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<&serde_json::Value>,
) {
    if let Some(value) = value.and_then(json_i64_value) {
        target.insert(key.to_string(), serde_json::Value::from(value));
    }
}

fn json_i64_value(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| value.as_f64().map(|value| value as i64))
}

fn restore_cloud_wordbook_preferences(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<usize, String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(0);
    };
    let mut selection = serde_json::json!({
        "1": false,
        "2": false,
        "3": false,
        "4": false
    });
    let Some(selection_obj) = selection.as_object_mut() else {
        return Ok(0);
    };
    let mut restored = 0usize;
    for item in items {
        let Some(wordbook_id) = item.get("wordbook_id").and_then(json_i64_value) else {
            continue;
        };
        let is_active = item
            .get("is_active")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        selection_obj.insert(wordbook_id.to_string(), serde_json::Value::Bool(is_active));
        restored += 1;
    }
    set_json_setting(conn, "saved_wordbooks_json", &selection)?;
    set_json_setting(conn, "today_wordbooks_json", &selection)?;
    set_json_setting(conn, "today_review_wordbooks_json", &selection)?;
    Ok(restored)
}

fn restore_cloud_report_snapshots(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<usize, String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(0);
    };
    let snapshots = items
        .iter()
        .filter_map(|item| {
            let snapshot_date = item
                .get("snapshot_date")
                .or_else(|| item.get("snapshotDate"))
                .and_then(|value| value.as_str())?;
            let payload_json = item
                .get("payload_json")
                .or_else(|| item.get("payloadJson"))?;
            Some(serde_json::json!({
                "snapshotDate": snapshot_date,
                "payloadJson": payload_json,
            }))
        })
        .collect::<Vec<_>>();
    if snapshots.is_empty() {
        return Ok(0);
    }
    set_json_setting(
        conn,
        "cloud_report_snapshots_json",
        &serde_json::Value::Array(snapshots.clone()),
    )?;
    Ok(snapshots.len())
}

fn restore_cloud_wrong_word_hints(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<usize, String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(0);
    };
    let mut restored = 0usize;
    for item in items {
        let Some(entry_id) = item
            .get("entry_id")
            .or_else(|| item.get("entryId"))
            .and_then(json_i64_value)
        else {
            continue;
        };
        if entry_id <= 0 {
            continue;
        }
        let hint_text = item
            .get("hint_text")
            .or_else(|| item.get("hintText"))
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        if hint_text.is_empty() {
            continue;
        }
        let entry_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM entries WHERE id = ?1)",
                [entry_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value != 0)
            .map_err(|e| format!("Failed to check restored hint entry {entry_id}: {e}"))?;
        if !entry_exists {
            continue;
        }
        let source = item
            .get("hint_source")
            .or_else(|| item.get("hintSource"))
            .and_then(|value| value.as_str())
            .unwrap_or("user");
        persistence::word_hint_repo::save_hint(conn, entry_id, hint_text, source)
            .map_err(|e| e.to_string())?;
        restored += 1;
    }
    Ok(restored)
}

fn restore_cloud_study_word_points(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<usize, String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(0);
    };
    let mut session_totals: BTreeMap<(String, String), i64> = BTreeMap::new();
    for point in items {
        let Some((date, mode, _, _, attempt_count, _, _, _, _)) =
            parse_cloud_study_word_point(point)
        else {
            continue;
        };
        *session_totals.entry((date, mode)).or_insert(0) += attempt_count.max(1);
    }
    for ((date, mode), total_words) in &session_totals {
        let session_id = cloud_restore_session_id(date, mode);
        let started_at = format!("{date}T00:00:00");
        let completed_at = format!("{date}T23:59:59");
        conn.execute(
            "INSERT OR REPLACE INTO study_sessions
             (session_id, mode, total_words, wordbook_id, started_at, completed_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5)",
            rusqlite::params![session_id, mode, total_words, started_at, completed_at],
        )
        .map_err(|e| format!("Failed to restore cloud study session: {e}"))?;
    }

    let mut restored_attempts = 0usize;
    for point in items {
        let Some((
            date,
            mode,
            entry_id,
            question_type,
            attempt_count,
            correct_count,
            wrong_count,
            total_response_time_ms,
            last_answered_at,
        )) = parse_cloud_study_word_point(point)
        else {
            continue;
        };
        let entry_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM entries WHERE id = ?1)",
                [entry_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value != 0)
            .map_err(|e| format!("Failed to check restored entry {entry_id}: {e}"))?;
        if !entry_exists {
            continue;
        }
        let attempts = attempt_count.max(correct_count + wrong_count).max(1);
        let response_time = if attempts > 0 {
            total_response_time_ms / attempts
        } else {
            0
        };
        let mut attempt_index = 0i64;
        for _ in 0..correct_count.max(0) {
            attempt_index += 1;
            insert_cloud_restored_study_result(
                conn,
                &date,
                &mode,
                entry_id,
                &question_type,
                attempt_index,
                "correct",
                response_time,
                &last_answered_at,
            )?;
            restored_attempts += 1;
        }
        for _ in 0..wrong_count.max(0) {
            attempt_index += 1;
            insert_cloud_restored_study_result(
                conn,
                &date,
                &mode,
                entry_id,
                &question_type,
                attempt_index,
                "incorrect",
                response_time,
                &last_answered_at,
            )?;
            restored_attempts += 1;
        }
        while attempt_index < attempts {
            attempt_index += 1;
            insert_cloud_restored_study_result(
                conn,
                &date,
                &mode,
                entry_id,
                &question_type,
                attempt_index,
                "skipped",
                response_time,
                &last_answered_at,
            )?;
            restored_attempts += 1;
        }
    }
    Ok(restored_attempts)
}

type CloudStudyWordPoint = (String, String, i64, String, i64, i64, i64, i64, String);

fn parse_cloud_study_word_point(point: &serde_json::Value) -> Option<CloudStudyWordPoint> {
    let date = point
        .get("point_date")
        .or_else(|| point.get("pointDate"))
        .and_then(|value| value.as_str())?
        .to_string();
    let mode = point
        .get("mode")
        .and_then(|value| value.as_str())?
        .to_string();
    let entry_id = point
        .get("entry_id")
        .or_else(|| point.get("entryId"))
        .and_then(json_i64_value)?;
    let question_type = normalize_cloud_question_type(
        point
            .get("question_type")
            .or_else(|| point.get("questionType"))
            .and_then(|value| value.as_str())
            .unwrap_or("enToCnChoice"),
    );
    let attempt_count = point
        .get("attempt_count")
        .or_else(|| point.get("attemptCount"))
        .and_then(json_i64_value)
        .unwrap_or(0);
    let correct_count = point
        .get("correct_count")
        .or_else(|| point.get("correctCount"))
        .and_then(json_i64_value)
        .unwrap_or(0);
    let wrong_count = point
        .get("wrong_count")
        .or_else(|| point.get("wrongCount"))
        .and_then(json_i64_value)
        .unwrap_or(0);
    let total_response_time_ms = point
        .get("total_response_time_ms")
        .or_else(|| point.get("totalResponseTimeMs"))
        .and_then(json_i64_value)
        .unwrap_or(0);
    let last_answered_at = point
        .get("last_answered_at")
        .or_else(|| point.get("lastAnsweredAt"))
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    if date.is_empty() || mode.is_empty() || entry_id <= 0 || attempt_count <= 0 {
        return None;
    }
    Some((
        date,
        mode,
        entry_id,
        question_type,
        attempt_count,
        correct_count,
        wrong_count,
        total_response_time_ms,
        last_answered_at,
    ))
}

fn normalize_cloud_question_type(value: &str) -> String {
    match normalize_persisted_enum_text(value).as_str() {
        "enToCnChoice" => "enToCnChoice".to_string(),
        "exampleToCnChoice" => "exampleToCnChoice".to_string(),
        "cnToEnChoice" => "cnToEnChoice".to_string(),
        "enToCnInput" => "enToCnInput".to_string(),
        "glossToRootInput" => "glossToRootInput".to_string(),
        "rootToGlossInput" => "rootToGlossInput".to_string(),
        "meaning" | "choice" | "unknown" => "enToCnChoice".to_string(),
        "spelling" | "input" => "enToCnInput".to_string(),
        _ => "enToCnChoice".to_string(),
    }
}

fn cloud_restore_session_id(date: &str, mode: &str) -> String {
    format!("cloud_restore:{date}:{mode}")
}

fn insert_cloud_restored_study_result(
    conn: &word_storage_core::Connection,
    date: &str,
    mode: &str,
    entry_id: i64,
    question_type: &str,
    attempt_index: i64,
    outcome: &str,
    response_time_ms: i64,
    last_answered_at: &str,
) -> Result<(), String> {
    let session_id = cloud_restore_session_id(date, mode);
    let question_id =
        format!("cloud_restore:{date}:{mode}:{entry_id}:{question_type}:{attempt_index}");
    let answered_at = if last_answered_at.trim().is_empty() {
        format!("{date}T12:00:00")
    } else {
        last_answered_at.to_string()
    };
    conn.execute(
        "INSERT OR REPLACE INTO study_results
         (session_id, question_id, entry_id, question_type, user_response,
          normalized_response, correct_answer, outcome, response_time_ms, answered_at)
         VALUES (?1, ?2, ?3, ?4, '', NULL, '', ?5, ?6, ?7)",
        rusqlite::params![
            session_id,
            question_id,
            entry_id,
            question_type,
            outcome,
            response_time_ms.max(0),
            answered_at,
        ],
    )
    .map_err(|e| format!("Failed to restore cloud study result: {e}"))?;
    Ok(())
}

pub fn get_ai_provider_config() -> Result<String, String> {
    with_runtime_conn(|conn| {
        ensure_ai_provider_config(conn)?;
        let summary = summarize_ai_provider_config(&resolve_ai_provider_config());
        serde_json::to_string(&summary).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn save_ai_provider_config(request_json: String) -> Result<String, String> {
    let request: StoredAiProviderConfig =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    with_runtime_conn(|conn| {
        ensure_ai_provider_config(conn)?;
        let value = serde_json::to_value(&request)
            .map_err(|e| format!("JSON serialization failed: {e}"))?;
        set_json_setting(conn, "ai_provider_config_json", &value)?;
        let summary = summarize_ai_provider_config(&request);
        serde_json::to_string(&summary).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

// ============================================================================
// Study Session API
// ============================================================================

use word_storage_core::models::{
    SessionMode, StartSessionEntryPayload, StartSessionMeaningPayload, StartSessionRequest,
    SubmitAnswerRequest,
};

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

    repair_seed_meaning_noise(&conn)?;

    let hydrated_request = hydrate_start_session_request(
        &conn,
        request,
        Some(runtime.paths().bundled_resource_path("")),
    )?;

    let response = core_start_study_session(&conn, hydrated_request)
        .map_err(|e| format!("Failed to start session: {}", e))?;

    let mut payload =
        serde_json::to_value(&response).map_err(|e| format!("JSON serialization failed: {e}"))?;
    enrich_study_response_hints(&conn, &mut payload)?;
    clean_json_string(&payload)
}

fn hydrate_start_session_request(
    conn: &word_storage_core::Connection,
    mut request: StartSessionRequest,
    bundle_resource_dir: Option<std::path::PathBuf>,
) -> Result<StartSessionRequest, String> {
    if !request.entry_source_ids.is_empty() || !request.entry_payloads.is_empty() {
        if request.distractor_payloads.is_empty() && !request.entry_payloads.is_empty() {
            let excluded_source_ids = request
                .entry_payloads
                .iter()
                .map(|payload| payload.source_id.as_str())
                .collect::<Vec<_>>();
            request.distractor_payloads =
                load_distractor_payloads_excluding_sources(conn, &excluded_source_ids, 96)?;
        }
        return Ok(request);
    }

    ensure_planning_state(conn)?;
    if let Some(bundle_dir) = bundle_resource_dir.as_deref() {
        ensure_seed_vocabulary_imported(conn, bundle_dir)?;
    }

    let target_count = study_mode_target_count(conn, &request.mode)?;
    if target_count == 0 {
        return Err(format!(
            "No target count configured for mode {:?}",
            request.mode
        ));
    }

    let active_wordbook_ids = selected_wordbook_ids_for_today(conn)?;
    let review_wordbook_ids = selected_review_wordbook_ids_for_today(conn)?;
    if matches!(request.mode, SessionMode::RootAffix) {
        let bundle_dir = bundle_resource_dir
            .as_deref()
            .ok_or_else(|| "Root/affix resources are unavailable".to_string())?;
        let payloads = load_root_affix_payloads_for_active_wordbooks(
            bundle_dir,
            &active_wordbook_ids,
            target_count,
        )?;
        if payloads.is_empty() {
            return Err("No root/affix entries available for the active wordbooks".to_string());
        }
        request.entry_source_ids = payloads
            .iter()
            .map(|payload| payload.source_id.clone())
            .collect();
        request.entry_payloads = payloads;
        request.distractor_payloads = Vec::new();
        return Ok(request);
    }

    let primary_entry_ids = match request.mode {
        SessionMode::Review => {
            load_review_entry_ids_for_today(conn, &review_wordbook_ids, target_count)?
        }
        SessionMode::MixedTest => load_random_entry_ids_for_wordbooks_with_global_fallback(
            conn,
            &active_wordbook_ids,
            target_count,
        )?,
        SessionMode::WrongWordReinforcement => {
            load_prioritized_wrong_word_entry_ids(conn, target_count)?
        }
        SessionMode::NewWord => load_unlearned_ranked_entry_ids_for_wordbooks_with_global_fallback(
            conn,
            &active_wordbook_ids,
            target_count,
        )?,
        SessionMode::RootAffix => unreachable!("root/affix mode handled above"),
    };

    let entry_ids = primary_entry_ids;

    let entry_payloads = load_entry_payloads(conn, &entry_ids)?;
    if entry_payloads.is_empty() {
        return Err("No study entries available from the local vocabulary data".to_string());
    }

    let distractor_target = std::cmp::max(target_count.saturating_mul(12), 96);
    let distractor_ids = load_ranked_entry_ids_for_wordbooks_excluding(
        conn,
        &active_wordbook_ids,
        &entry_ids,
        distractor_target,
    )?;
    let distractor_payloads = load_entry_payloads(conn, &distractor_ids)?;

    request.wordbook_id = request.wordbook_id.or_else(|| match request.mode {
        SessionMode::Review => review_wordbook_ids.first().copied(),
        _ => active_wordbook_ids.first().copied(),
    });
    request.entry_source_ids = entry_payloads
        .iter()
        .map(|payload| payload.source_id.clone())
        .collect();
    request.entry_payloads = entry_payloads.clone();
    request.distractor_payloads = distractor_payloads;

    Ok(request)
}

fn study_mode_target_count(
    conn: &word_storage_core::Connection,
    mode: &SessionMode,
) -> Result<usize, String> {
    let plan = get_json_setting(conn, "today_plan_json", &default_plan_json())?;
    let today = today_date_string();
    let count = match mode {
        SessionMode::NewWord => grown_plan_unit_count(&plan, "newWord", "newWordsPerDay", &today),
        SessionMode::Review => grown_plan_unit_count(&plan, "review", "reviewWordsPerDay", &today),
        SessionMode::MixedTest => {
            grown_plan_unit_count(&plan, "mixedTest", "mixedTestPerDay", &today)
        }
        SessionMode::WrongWordReinforcement => grown_plan_unit_count(
            &plan,
            "wrongWordReinforcement",
            "wrongWordTestPerDay",
            &today,
        ),
        SessionMode::RootAffix => {
            grown_plan_unit_count(&plan, "rootAffix", "rootAffixPerDay", &today)
        }
    };
    Ok(count as usize)
}

fn selected_wordbook_ids_for_today(
    conn: &word_storage_core::Connection,
) -> Result<Vec<i64>, String> {
    let today_selection = get_json_setting(
        conn,
        "today_wordbooks_json",
        &default_wordbook_selection_json(),
    )?;
    Ok(active_wordbook_ids_from_selection(&today_selection))
}

fn selected_review_wordbook_ids_for_today(
    conn: &word_storage_core::Connection,
) -> Result<Vec<i64>, String> {
    let today_selection = get_json_setting(
        conn,
        "today_wordbooks_json",
        &default_wordbook_selection_json(),
    )?;
    let review_selection = get_json_setting(conn, "today_review_wordbooks_json", &today_selection)?;
    let review_ids = active_wordbook_ids_from_selection(&review_selection);
    if !review_ids.is_empty() {
        let today_ids = active_wordbook_ids_from_selection(&today_selection);
        if !has_review_candidates_for_wordbooks(conn, &review_ids)?
            && !today_ids.is_empty()
            && has_review_candidates_for_wordbooks(conn, &today_ids)?
        {
            return Ok(today_ids);
        }
        return Ok(review_ids);
    }
    Ok(active_wordbook_ids_from_selection(&today_selection))
}

fn active_wordbook_ids_from_selection(selection: &serde_json::Value) -> Vec<i64> {
    let Some(object) = selection.as_object() else {
        return Vec::new();
    };
    let mut ids = object
        .iter()
        .filter_map(|(key, value)| {
            if value.as_bool() == Some(true) {
                key.parse::<i64>().ok()
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

fn load_prioritized_wrong_word_entry_ids(
    conn: &word_storage_core::Connection,
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mut entries = load_wrong_word_entries(conn)?;
    let ordered = wrong_words_domain::build_wrong_words(&mut entries, "all");
    Ok(ordered
        .into_iter()
        .take(limit)
        .filter_map(|item| item.get("entryId").and_then(|value| value.as_i64()))
        .filter(|entry_id| *entry_id > 0)
        .collect())
}

#[derive(Clone)]
struct RootAffixCard {
    id: String,
    form: String,
    meaning_cn: String,
    example_pairs: Vec<(String, String)>,
    scope: String,
}

fn load_root_affix_payloads_for_active_wordbooks(
    bundle_dir: &Path,
    active_wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<StartSessionEntryPayload>, String> {
    load_root_affix_payloads_for_active_wordbooks_on_date(
        bundle_dir,
        active_wordbook_ids,
        limit,
        &today_date_string(),
    )
}

fn load_root_affix_payloads_for_active_wordbooks_on_date(
    bundle_dir: &Path,
    active_wordbook_ids: &[i64],
    limit: usize,
    today_date: &str,
) -> Result<Vec<StartSessionEntryPayload>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let all = load_root_affix_cards(bundle_dir)?;
    let include_medical = active_wordbook_ids.contains(&4);
    let include_shared = active_wordbook_ids.iter().any(|id| matches!(id, 1 | 2 | 3));
    let mut scoped = all
        .iter()
        .filter(|card| {
            is_reliable_root_affix_card(card)
                && ((card.scope == "medical" && include_medical)
                    || (card.scope == "shared" && include_shared))
        })
        .cloned()
        .collect::<Vec<_>>();
    if scoped.is_empty() && !include_medical && !include_shared {
        scoped = all
            .into_iter()
            .filter(is_reliable_root_affix_card)
            .collect::<Vec<_>>();
    }
    let seed = daily_selection_seed(today_date) as u64;
    scoped.sort_by(|left, right| {
        let left_key = stable_daily_sort_key(&left.id, seed);
        let right_key = stable_daily_sort_key(&right.id, seed);
        left_key
            .cmp(&right_key)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(scoped
        .into_iter()
        .take(limit)
        .map(root_affix_card_to_payload)
        .collect())
}

fn load_root_affix_cards(bundle_dir: &Path) -> Result<Vec<RootAffixCard>, String> {
    let mut cards: Vec<RootAffixCard> = Vec::new();
    cards.extend(load_shared_root_affix_cards(bundle_dir)?);
    cards.extend(load_medical_root_affix_cards(bundle_dir)?);
    Ok(cards)
}

fn load_shared_root_affix_cards(bundle_dir: &Path) -> Result<Vec<RootAffixCard>, String> {
    let book_dir = bundle_dir.join("seed-vocab").join("book");
    let mut by_id = BTreeMap::<String, RootAffixCard>::new();
    if !book_dir.exists() {
        return Ok(Vec::new());
    }
    for entry in fs::read_dir(&book_dir).map_err(|e| format!("Failed to read book assets: {e}"))? {
        let path = entry
            .map_err(|e| format!("Failed to read book asset entry: {e}"))?
            .path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.ends_with(".json") || name == "MEDICAL_RESP.json" {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read root/affix book asset {name}: {e}"))?;
        let items = serde_json::from_str::<Vec<serde_json::Value>>(&raw)
            .map_err(|e| format!("Invalid root/affix book asset {name}: {e}"))?;
        for item in items {
            let word = item
                .pointer("/content/word")
                .and_then(|value| value.as_object());
            let display_word = word
                .and_then(|value| value.get("wordHead"))
                .and_then(|value| value.as_str())
                .or_else(|| item.get("headWord").and_then(|value| value.as_str()))
                .unwrap_or("");
            let content = item.pointer("/content/word/content");
            let gloss = primary_meaning_from_content(content);
            let rem_method = content
                .and_then(|value| value.pointer("/remMethod/val"))
                .and_then(|value| value.as_str())
                .unwrap_or("");
            for card in parse_shared_root_affix_cards(display_word, &gloss, rem_method) {
                merge_root_affix_card(&mut by_id, card);
            }
        }
    }
    Ok(by_id
        .into_values()
        .filter(is_reliable_root_affix_card)
        .collect())
}

fn parse_shared_root_affix_cards(word: &str, gloss: &str, rem_method: &str) -> Vec<RootAffixCard> {
    if rem_method.trim().is_empty() || word.len() < 3 {
        return Vec::new();
    }
    let formula = trim_before_arrow(rem_method);
    let matcher = regex::Regex::new(r"([A-Za-z-]{2,12})\s*\(([^)]+)\)").unwrap();
    let matches = matcher
        .captures_iter(&formula)
        .filter_map(|cap| Some((cap.get(1)?.as_str().trim(), cap.get(2)?.as_str().trim())))
        .collect::<Vec<_>>();
    let lower_word = word.to_lowercase();
    let mut cards: Vec<RootAffixCard> = Vec::new();
    for (index, (form, meaning)) in matches.iter().enumerate() {
        let meaning = sanitize_chinese_meaning(meaning);
        let normalized = normalize_root_affix_form(form);
        if normalized.len() < 2 || normalized.len() >= lower_word.len() || meaning.is_empty() {
            continue;
        }
        if !lower_word.contains(&normalized) && !form.starts_with('-') && !form.ends_with('-') {
            continue;
        }
        let display_form = if form.starts_with('-') || form.ends_with('-') {
            (*form).to_string()
        } else {
            format_shared_root_affix_form(&lower_word, &normalized, index, matches.len())
        };
        cards.push(RootAffixCard {
            id: format!("root_affix_shared_{normalized}"),
            form: display_form,
            meaning_cn: meaning,
            example_pairs: vec![(
                word.to_string(),
                compact_root_example_gloss(&sanitize_chinese_meaning(gloss)),
            )],
            scope: "shared".to_string(),
        });
    }
    cards
}

fn load_medical_root_affix_cards(bundle_dir: &Path) -> Result<Vec<RootAffixCard>, String> {
    let path = bundle_dir
        .join("seed-medical")
        .join("medical-root-affix.txt");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read medical root/affix asset: {e}"))?;
    let mut cards: Vec<RootAffixCard> = Vec::new();
    let mut last_index: Option<usize> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.contains("常见") || trimmed.starts_with('四') {
            continue;
        }
        if let Some(payload) = trimmed
            .strip_prefix("例：")
            .or_else(|| trimmed.strip_prefix("例:"))
        {
            if let Some(index) = last_index {
                cards[index].example_pairs = split_medical_examples(payload);
            }
            continue;
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 2 {
            continue;
        }
        let form = parts[0].trim();
        let meaning = parts[1..].join(" ");
        let normalized = normalize_root_affix_form(form);
        if normalized.is_empty() || !contains_han(&meaning) {
            continue;
        }
        cards.push(RootAffixCard {
            id: format!("root_affix_medical_{normalized}"),
            form: form.to_string(),
            meaning_cn: meaning,
            example_pairs: Vec::new(),
            scope: "medical".to_string(),
        });
        last_index = cards.len().checked_sub(1);
    }
    Ok(cards)
}

fn root_affix_card_to_payload(card: RootAffixCard) -> StartSessionEntryPayload {
    let (example_words, example_glosses) = limited_root_example_text(&card.example_pairs, 3);
    StartSessionEntryPayload {
        source_id: card.id,
        word: card.form,
        part_of_speech: None,
        frequency: 0.0,
        phonetic_us: None,
        phonetic_uk: None,
        meaning_details: vec![StartSessionMeaningPayload {
            pos: "root".to_string(),
            meaning_cn: card.meaning_cn.clone(),
            meaning_en: None,
        }],
        meanings: vec![card.meaning_cn],
        example_sentence: non_empty_string(example_words),
        example_translation: non_empty_string(example_glosses),
    }
}

fn non_empty_string(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}

fn merge_root_affix_card(by_id: &mut BTreeMap<String, RootAffixCard>, incoming: RootAffixCard) {
    if let Some(existing) = by_id.get_mut(&incoming.id) {
        merge_root_example_pairs(&mut existing.example_pairs, incoming.example_pairs);
    } else {
        by_id.insert(incoming.id.clone(), incoming);
    }
}

fn merge_root_example_pairs(current: &mut Vec<(String, String)>, incoming: Vec<(String, String)>) {
    let mut seen = current
        .iter()
        .map(|(word, _)| word.trim().to_lowercase())
        .collect::<BTreeSet<_>>();
    for (word, gloss) in incoming {
        let word = word.trim();
        if word.is_empty() {
            continue;
        }
        if seen.insert(word.to_lowercase()) {
            current.push((word.to_string(), compact_root_example_gloss(&gloss)));
        }
    }
}

fn limited_root_example_text(pairs: &[(String, String)], limit: usize) -> (String, String) {
    let selected = pairs
        .iter()
        .filter(|(word, _)| !word.trim().is_empty())
        .take(limit)
        .collect::<Vec<_>>();
    let words = selected
        .iter()
        .map(|(word, _)| word.trim())
        .collect::<Vec<_>>()
        .join(", ");
    let glosses = selected
        .iter()
        .map(|(_, gloss)| compact_root_example_gloss(gloss))
        .collect::<Vec<_>>()
        .join(", ");
    (words, glosses)
}

fn compact_root_example_gloss(value: &str) -> String {
    let mut parts = Vec::new();
    let mut seen = BTreeSet::new();
    for raw in value.split(&[';', '；', ',', '，', '、', '/'][..]) {
        let cleaned = sanitize_chinese_meaning(raw);
        if cleaned.is_empty() || cleaned.chars().count() > 12 {
            continue;
        }
        if seen.insert(cleaned.clone()) {
            parts.push(cleaned);
        }
        if parts.len() >= 3 {
            break;
        }
    }
    if parts.is_empty() {
        sanitize_chinese_meaning(value).chars().take(18).collect()
    } else {
        parts.join("；")
    }
}

fn primary_meaning_from_content(content: Option<&serde_json::Value>) -> String {
    content
        .and_then(|value| value.pointer("/trans/0/tranCn"))
        .and_then(|value| value.as_str())
        .map(sanitize_chinese_meaning)
        .unwrap_or_default()
}

fn trim_before_arrow(value: &str) -> String {
    for marker in ["→", "->", "鈫"].iter() {
        if let Some(index) = value.find(marker) {
            return value[..index].to_string();
        }
    }
    value.to_string()
}

fn format_shared_root_affix_form(
    word: &str,
    normalized: &str,
    index: usize,
    total: usize,
) -> String {
    if index == 0 && total > 1 && word.starts_with(normalized) {
        format!("{normalized}-")
    } else if index + 1 == total && total > 1 && word.ends_with(normalized) {
        format!("-{normalized}")
    } else {
        normalized.to_string()
    }
}

fn sanitize_chinese_meaning(value: &str) -> String {
    value
        .trim()
        .trim_matches(|ch| matches!(ch, '[' | ']' | '，' | ',' | ';' | '；' | ' ' | '　'))
        .to_string()
}

fn contains_han(value: &str) -> bool {
    value
        .chars()
        .any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch))
}

fn normalize_root_affix_form(form: &str) -> String {
    form.chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .collect::<String>()
        .to_lowercase()
}

fn is_reliable_root_affix_card(card: &RootAffixCard) -> bool {
    let normalized = normalize_root_affix_form(&card.form);
    if normalized.len() < 2 || normalized.len() > 6 {
        return false;
    }
    if card.scope == "shared" && normalized.len() > 4 {
        return false;
    }
    if !contains_han(&card.meaning_cn) || sanitize_chinese_meaning(&card.meaning_cn).is_empty() {
        return false;
    }
    let distinct_examples = card
        .example_pairs
        .iter()
        .filter(|(word, _)| !word.trim().is_empty())
        .map(|(word, _)| word.trim().to_lowercase())
        .collect::<BTreeSet<_>>()
        .len();
    distinct_examples >= 2
}

fn split_medical_examples(payload: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut seen = BTreeSet::new();
    for item in payload.split(&['；', ';', ','][..]) {
        let trimmed = item.trim();
        let parsed = trimmed
            .split_once('（')
            .and_then(|(word, rest)| rest.split_once('）').map(|(gloss, _)| (word, gloss)))
            .or_else(|| {
                trimmed
                    .split_once('(')
                    .and_then(|(word, rest)| rest.split_once(')').map(|(gloss, _)| (word, gloss)))
            });
        if let Some((word, gloss)) = parsed {
            let word = word.trim();
            if word.is_empty() {
                continue;
            }
            if seen.insert(word.to_lowercase()) {
                pairs.push((word.to_string(), compact_root_example_gloss(gloss)));
            }
        }
    }
    pairs
}

fn load_learned_entry_ids_for_wordbooks(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mut conditions = "sr.outcome NOT IN ('skipped', '\"skipped\"')".to_string();
    let wordbook_join = if wordbook_ids.is_empty() {
        String::new()
    } else {
        let placeholders = wordbook_ids
            .iter()
            .enumerate()
            .map(|(index, _)| format!("?{}", index + 1))
            .collect::<Vec<_>>()
            .join(", ");
        conditions.push_str(&format!(" AND we.wordbook_id IN ({placeholders})"));
        "JOIN wordbook_entries we ON we.entry_id = e.id".to_string()
    };
    let sql = format!(
        "SELECT e.id,
                MIN(sr.answered_at) AS first_learned_at,
                MAX(sr.answered_at) AS last_answered_at,
                e.frequency
         FROM study_results sr
         JOIN entries e ON e.id = sr.entry_id OR e.source_entry_key = CAST(sr.entry_id AS TEXT)
         {wordbook_join}
         WHERE {conditions}
         GROUP BY e.id
         ORDER BY first_learned_at ASC, e.frequency DESC, e.id ASC"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare learned entry seed query: {e}"))?;
    let param_refs = wordbook_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();
    let today = today_date_string();
    let rows = stmt
        .query_map(&param_refs[..], |row| {
            Ok(ReviewCandidate {
                entry_id: row.get(0)?,
                first_learned_at: row.get(1)?,
                last_answered_at: row.get(2)?,
                frequency: row.get(3)?,
            })
        })
        .map_err(|e| format!("Failed to query learned entry seed: {e}"))?;
    let candidates = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode learned entry seed row: {e}"))?;
    Ok(select_review_candidates(candidates, &today, limit))
}

fn load_review_entry_ids_for_today(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let ids = load_learned_entry_ids_for_wordbooks(conn, wordbook_ids, limit)?;
    if !ids.is_empty() || wordbook_ids.is_empty() {
        return Ok(ids);
    }
    load_learned_entry_ids_for_wordbooks(conn, &[], limit)
}

#[derive(Debug)]
struct ReviewCandidate {
    entry_id: i64,
    first_learned_at: String,
    last_answered_at: String,
    frequency: f64,
}

fn select_review_candidates(
    candidates: Vec<ReviewCandidate>,
    today_date: &str,
    limit: usize,
) -> Vec<i64> {
    if limit == 0 {
        return Vec::new();
    }
    let today = chrono::NaiveDate::parse_from_str(today_date, "%Y-%m-%d").ok();
    let mut eligible = candidates
        .into_iter()
        .filter_map(|candidate| {
            let first_date = local_date_from_rfc3339(&candidate.first_learned_at)?;
            if first_date.as_str() >= today_date {
                return None;
            }
            let age_days = today
                .and_then(|today| {
                    chrono::NaiveDate::parse_from_str(&first_date, "%Y-%m-%d")
                        .ok()
                        .map(|first| (today - first).num_days())
                })
                .unwrap_or(1)
                .max(1);
            let last_date =
                local_date_from_rfc3339(&candidate.last_answered_at).unwrap_or(first_date);
            Some((candidate, age_days, last_date))
        })
        .collect::<Vec<_>>();

    let mut buckets = [
        Vec::<(ReviewCandidate, i64, String)>::new(),
        Vec::<(ReviewCandidate, i64, String)>::new(),
        Vec::<(ReviewCandidate, i64, String)>::new(),
        Vec::<(ReviewCandidate, i64, String)>::new(),
        Vec::<(ReviewCandidate, i64, String)>::new(),
        Vec::<(ReviewCandidate, i64, String)>::new(),
    ];
    for item in eligible.drain(..) {
        let bucket = review_age_bucket(item.1) as usize;
        buckets[bucket].push(item);
    }
    for bucket in buckets.iter_mut() {
        bucket.sort_by(|left, right| {
            left.2
                .cmp(&right.2)
                .then_with(|| right.0.frequency.total_cmp(&left.0.frequency))
                .then_with(|| left.0.entry_id.cmp(&right.0.entry_id))
        });
    }

    let weights = [4usize, 3, 2, 2, 1, 1];
    let total_weight = weights.iter().sum::<usize>();
    let mut selected = Vec::with_capacity(limit);
    let mut used = BTreeSet::new();

    for (index, weight) in weights.iter().enumerate() {
        if selected.len() >= limit {
            break;
        }
        let quota = ((limit * weight) + total_weight - 1) / total_weight;
        take_review_bucket_items(&mut selected, &mut used, &buckets[index], quota, limit);
    }
    for bucket in &buckets {
        if selected.len() >= limit {
            break;
        }
        take_review_bucket_items(&mut selected, &mut used, bucket, limit, limit);
    }
    selected
}

fn take_review_bucket_items(
    selected: &mut Vec<i64>,
    used: &mut BTreeSet<i64>,
    bucket: &[(ReviewCandidate, i64, String)],
    quota: usize,
    limit: usize,
) {
    let mut added = 0usize;
    let target = quota.min(limit.saturating_sub(selected.len()));
    for item in bucket.iter() {
        if used.insert(item.0.entry_id) {
            selected.push(item.0.entry_id);
            added += 1;
            if added >= target || selected.len() >= limit {
                break;
            }
        }
    }
}

fn review_age_bucket(age_days: i64) -> i64 {
    match age_days {
        0..=1 => 0,
        2..=3 => 1,
        4..=7 => 2,
        8..=14 => 3,
        15..=30 => 4,
        _ => 5,
    }
}

fn load_unlearned_ranked_entry_ids_for_wordbooks_with_global_fallback(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let ids = load_unlearned_ranked_entry_ids_for_wordbooks(conn, wordbook_ids, limit)?;
    if ids.is_empty() && wordbook_ids.is_empty() {
        load_unlearned_ranked_entry_ids(conn, limit)
    } else {
        Ok(ids)
    }
}

fn load_unlearned_ranked_entry_ids_for_wordbooks(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    load_unlearned_ranked_entry_ids_for_wordbooks_on_date(
        conn,
        wordbook_ids,
        limit,
        &today_date_string(),
    )
}

fn load_unlearned_ranked_entry_ids_for_wordbooks_on_date(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
    today_date: &str,
) -> Result<Vec<i64>, String> {
    if wordbook_ids.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let seed = daily_selection_seed(today_date);
    let placeholders = wordbook_ids
        .iter()
        .enumerate()
        .map(|(index, _)| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT DISTINCT we.entry_id
         FROM wordbook_entries we
         JOIN entries e ON e.id = we.entry_id
         WHERE we.wordbook_id IN ({placeholders})
           AND NOT EXISTS (
             SELECT 1
             FROM study_results sr
             WHERE sr.entry_id = e.id
           )
         ORDER BY ((we.rank_in_book * ?{}) % 9973) ASC,
                  we.rank_in_book ASC,
                  e.id ASC
         LIMIT {limit}",
        wordbook_ids.len() + 1
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare unlearned wordbook entry seed query: {e}"))?;
    let mut params = wordbook_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();
    params.push(&seed);
    let rows = stmt
        .query_map(&params[..], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query unlearned wordbook entry seed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode unlearned wordbook entry seed row: {e}"))
}

fn daily_selection_seed(today_date: &str) -> i64 {
    let mut hash = 1469598103934665603u64;
    for byte in today_date.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    (hash % 9973 + 1) as i64
}

fn stable_daily_sort_key(value: &str, seed: u64) -> u64 {
    let mut hash = 1469598103934665603u64 ^ seed;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn load_unlearned_ranked_entry_ids(
    conn: &word_storage_core::Connection,
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let sql = format!(
        "SELECT e.id
         FROM entries e
         WHERE NOT EXISTS (
           SELECT 1
           FROM study_results sr
           WHERE sr.entry_id = e.id
         )
         ORDER BY e.frequency DESC, e.id ASC
         LIMIT {limit}"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare unlearned fallback entry seed query: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query unlearned fallback entry seed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode unlearned fallback entry seed row: {e}"))
}

fn load_ranked_entry_ids_for_wordbooks_with_global_fallback(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let ids = load_ranked_entry_ids_for_wordbooks(conn, wordbook_ids, limit)?;
    if ids.is_empty() && wordbook_ids.is_empty() {
        load_ranked_entry_ids(conn, limit)
    } else {
        Ok(ids)
    }
}

fn load_random_entry_ids_for_wordbooks_with_global_fallback(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let ids = load_random_entry_ids_for_wordbooks(conn, wordbook_ids, limit)?;
    if ids.is_empty() && wordbook_ids.is_empty() {
        load_random_entry_ids(conn, limit)
    } else {
        Ok(ids)
    }
}

fn load_random_entry_ids_for_wordbooks(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    if wordbook_ids.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let placeholders = wordbook_ids
        .iter()
        .enumerate()
        .map(|(index, _)| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT DISTINCT we.entry_id
         FROM wordbook_entries we
         JOIN entries e ON e.id = we.entry_id
         WHERE we.wordbook_id IN ({placeholders})
         ORDER BY RANDOM()
         LIMIT {limit}"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare random wordbook entry seed query: {e}"))?;
    let param_refs = wordbook_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();
    let rows = stmt
        .query_map(&param_refs[..], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query random wordbook entry seed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode random wordbook entry seed row: {e}"))
}

fn load_ranked_entry_ids_for_wordbooks(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    if wordbook_ids.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    let placeholders = wordbook_ids
        .iter()
        .enumerate()
        .map(|(index, _)| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "SELECT DISTINCT we.entry_id
         FROM wordbook_entries we
         JOIN entries e ON e.id = we.entry_id
         WHERE we.wordbook_id IN ({placeholders})
         ORDER BY we.rank_in_book ASC, e.frequency DESC, e.id ASC
         LIMIT {limit}"
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare wordbook entry seed query: {e}"))?;
    let param_refs = wordbook_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();
    let rows = stmt
        .query_map(&param_refs[..], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query wordbook entry seed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode wordbook entry seed row: {e}"))
}

fn load_ranked_entry_ids_for_wordbooks_excluding(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    excluded_entry_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    let ranked = if wordbook_ids.is_empty() {
        load_ranked_entry_ids(conn, limit.saturating_add(excluded_entry_ids.len()))?
    } else {
        load_ranked_entry_ids_for_wordbooks(
            conn,
            wordbook_ids,
            limit.saturating_add(excluded_entry_ids.len()),
        )?
    };
    Ok(ranked
        .into_iter()
        .filter(|id| !excluded_entry_ids.contains(id))
        .take(limit)
        .collect())
}

fn load_distractor_payloads_excluding_sources(
    conn: &word_storage_core::Connection,
    excluded_source_ids: &[&str],
    limit: usize,
) -> Result<Vec<StartSessionEntryPayload>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let excluded_entry_ids = load_entry_ids_for_source_keys(conn, excluded_source_ids)?;
    let active_wordbook_ids = selected_wordbook_ids_for_today(conn)?;
    let mut distractor_ids = load_ranked_entry_ids_for_wordbooks_excluding(
        conn,
        &active_wordbook_ids,
        &excluded_entry_ids,
        limit,
    )?;
    if distractor_ids.len() < limit {
        for id in load_ranked_entry_ids(conn, limit.saturating_mul(2))? {
            if excluded_entry_ids.contains(&id) || distractor_ids.contains(&id) {
                continue;
            }
            distractor_ids.push(id);
            if distractor_ids.len() >= limit {
                break;
            }
        }
    }
    load_entry_payloads(conn, &distractor_ids)
}

fn load_entry_ids_for_source_keys(
    conn: &word_storage_core::Connection,
    source_keys: &[&str],
) -> Result<Vec<i64>, String> {
    let mut ids = Vec::new();
    for source_key in source_keys {
        let maybe_id = conn
            .query_row(
                "SELECT id FROM entries WHERE source_entry_key = ?1",
                [source_key],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| format!("Failed to query source entry id: {e}"))?;
        if let Some(id) = maybe_id {
            ids.push(id);
        }
    }
    Ok(ids)
}

fn load_ranked_entry_ids(
    conn: &word_storage_core::Connection,
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let sql = format!(
        "SELECT id
         FROM entries
         ORDER BY frequency DESC, id ASC
         LIMIT {limit}"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare fallback entry seed query: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query fallback entry seed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode fallback entry seed row: {e}"))
}

fn load_random_entry_ids(
    conn: &word_storage_core::Connection,
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let sql = format!(
        "SELECT id
         FROM entries
         ORDER BY RANDOM()
         LIMIT {limit}"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare random fallback entry seed query: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query random fallback entry seed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode random fallback entry seed row: {e}"))
}

fn load_entry_payloads(
    conn: &word_storage_core::Connection,
    entry_ids: &[i64],
) -> Result<Vec<StartSessionEntryPayload>, String> {
    let mut payloads = Vec::with_capacity(entry_ids.len());
    for entry_id in entry_ids {
        let payload = load_entry_payload(conn, *entry_id)?;
        payloads.push(payload);
    }
    Ok(payloads)
}

fn load_entry_payload(
    conn: &word_storage_core::Connection,
    entry_id: i64,
) -> Result<StartSessionEntryPayload, String> {
    let (source_id, word, part_of_speech, phonetic_us, phonetic_uk, frequency) = conn
        .query_row(
            "SELECT source_entry_key, word, part_of_speech, phonetic_us, phonetic_uk, frequency
             FROM entries
             WHERE id = ?1",
            [entry_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, f64>(5)?,
                ))
            },
        )
        .map_err(|e| format!("Failed to load study entry payload: {e}"))?;

    let meaning_details = load_entry_meaning_details(conn, entry_id)?
        .into_iter()
        .map(|value| {
            serde_json::from_value::<StartSessionMeaningPayload>(value)
                .map_err(|e| format!("Invalid study meaning payload: {e}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let meanings = meaning_details
        .iter()
        .map(|meaning| meaning.meaning_cn.clone())
        .collect::<Vec<_>>();
    let examples = load_entry_examples(conn, entry_id)?;
    let example_sentence = examples
        .first()
        .and_then(|value| value.get("sentenceEn"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let example_translation = examples
        .first()
        .and_then(|value| value.get("sentenceCn"))
        .and_then(|value| value.as_str())
        .map(str::to_string);

    Ok(StartSessionEntryPayload {
        source_id: source_id.clone(),
        word,
        part_of_speech,
        frequency,
        phonetic_us,
        phonetic_uk,
        meaning_details,
        meanings,
        example_sentence,
        example_translation,
    })
}

/// Submit a study answer.
///
/// Takes a JSON string containing SubmitAnswerRequest.
/// Returns a JSON string containing SubmitAnswerResponse.
pub fn submit_study_answer(request_json: String) -> Result<String, String> {
    let request: SubmitAnswerRequest =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;

    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    repair_seed_meaning_noise(&conn)?;

    let response = core_submit_study_answer(&conn, request)
        .map_err(|e| format!("Failed to submit answer: {}", e))?;
    enqueue_recent_study_word_points(&conn);
    enqueue_wrong_word_entries_snapshot(&conn);

    let mut payload =
        serde_json::to_value(&response).map_err(|e| format!("JSON serialization failed: {e}"))?;
    enrich_submit_response_hints(&conn, &mut payload)?;
    clean_json_string(&payload)
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

    let response = core_complete_study_session(&conn, &session_id)
        .map_err(|e| format!("Failed to complete session: {}", e))?;
    enqueue_recent_study_word_points(&conn);
    enqueue_wrong_word_entries_snapshot(&conn);

    serde_json::to_string(&response).map_err(|e| format!("JSON serialization failed: {}", e))
}

/// Cancel one study session by id.
pub fn cancel_study_session(session_id: String) -> Result<(), String> {
    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    core_cancel_study_session(&conn, &session_id)
        .map_err(|e| format!("Failed to cancel session: {}", e))
}

#[cfg(test)]
mod tests {
    use super::{
        active_wordbook_ids_from_selection, align_today_targets_to_available_pools,
        analyze_wrong_word_image_with_ai, build_ai_user_prompt,
        build_authoritative_today_home_state, build_hint_prompt_payload,
        build_wrong_word_entries_payload, clear_persisted_active_study_sessions,
        commit_wrong_word_import_with_connection, enrich_root_affix_entries_from_assets,
        enrich_submit_response_hints, ensure_planning_state, ensure_seed_vocabulary_imported,
        find_ai_passage_for_date, get_json_setting, hydrate_start_session_request,
        load_learned_entry_ids_for_wordbooks, load_prioritized_wrong_word_entry_ids,
        load_reports_history, load_review_entry_ids_for_today,
        load_root_affix_payloads_for_active_wordbooks_on_date, load_today_completion_seed,
        load_unlearned_ranked_entry_ids_for_wordbooks_on_date,
        load_wrong_word_detail_payload_with_bundle, load_wrong_word_entries,
        load_wrong_word_inputs, normalize_stored_session_mode, parse_ai_model_output,
        parse_wrong_word_import_ai_output, recommendation_library_for_word, recompute_summary_json,
        repair_seed_vocabulary_dedup, reset_user_owned_local_data, resolve_ai_request_wrong_words,
        restore_cloud_wordbook_preferences, restore_cloud_wrong_word_hints,
        select_review_candidates, selected_review_wordbook_ids_for_today,
        selected_wordbook_ids_for_today, set_json_setting, should_replace_today_review_wordbooks,
        single_wordbook_selection_json, today_date_string, today_target_seed_from_plan_value,
        today_target_seed_from_plan_value_for_date, ReviewCandidate, AI_PASSAGE_MAX_WRONG_WORDS,
        WORD_HINT_RECOMMENDATIONS_JSON,
    };
    use crate::ai_agent::{AiAgent, AiProviderConfig, AiProviderProfile};
    use rusqlite::Connection;
    use std::env;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let original = env::var(key).ok();
            unsafe { env::set_var(key, value) };
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                unsafe { env::set_var(self.key, value) };
            } else {
                unsafe { env::remove_var(self.key) };
            }
        }
    }

    fn spawn_json_server(status_line: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("test server addr");
        let status_line = status_line.to_string();
        let body = body.to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept test request");
            let mut buffer = [0u8; 2048];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write test response");
        });
        format!("http://{addr}")
    }

    fn seed_basic_entry(conn: &Connection, word: &str, meaning: &str) -> i64 {
        conn.execute(
            "INSERT OR IGNORE INTO study_sessions (session_id, mode, total_words, started_at)
             VALUES ('s1', 'newWord', 1, '2026-05-04T00:00:00Z')",
            [],
        )
        .expect("insert session");
        conn.execute(
            "INSERT OR IGNORE INTO source_versions (source_name, source_commit, status)
             VALUES ('test', 'hint-bridge-test-v1', 'ready')",
            [],
        )
        .expect("insert source version");
        let source_version_id: i64 = conn
            .query_row(
                "SELECT id FROM source_versions WHERE source_commit = 'hint-bridge-test-v1'",
                [],
                |row| row.get(0),
            )
            .expect("load source version");
        conn.execute(
            "INSERT INTO entries
             (source_version_id, source_entry_key, word, lemma, part_of_speech)
             VALUES (?1, ?2, ?3, ?3, 'adj')",
            rusqlite::params![source_version_id, format!("test_{word}"), word],
        )
        .expect("insert entry");
        let entry_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, meaning_cn, sort_order)
             VALUES (?1, ?2, 0)",
            rusqlite::params![entry_id, meaning],
        )
        .expect("insert meaning");
        entry_id
    }

    #[test]
    fn primary_failure_falls_back_directly_to_openai_backup() {
        let primary_url = spawn_json_server("500 Internal Server Error", r#"{"error":"down"}"#);
        let backup_url = spawn_json_server(
            "200 OK",
            r#"{"output":[{"content":[{"type":"output_text","text":"{\"title\":\"fallback\",\"paragraphs\":[\"ok\"]}"}]}]}"#,
        );
        let _primary_url = EnvGuard::set("ANTHROPIC_BASE_URL", &primary_url);
        let _primary_key = EnvGuard::set("ANTHROPIC_AUTH_TOKEN", "primary-test-key");
        let _backup_url = EnvGuard::set("OPENAI_BASE_URL", &backup_url);
        let _backup_key = EnvGuard::set("OPENAI_AUTH_TOKEN", "backup-test-key");
        let _backup_model = EnvGuard::set("OPENAI_MODEL", "gpt-test");

        let agent = AiAgent::new(AiProviderConfig {
            primary: AiProviderProfile {
                provider: "anthropic".to_string(),
                base_url: primary_url,
                model: "claude-test".to_string(),
                auth_token: "primary-test-key".to_string(),
            },
            backup: AiProviderProfile {
                provider: "openaiResponses".to_string(),
                base_url: backup_url,
                model: "gpt-test".to_string(),
                auth_token: "backup-test-key".to_string(),
            },
        });

        let result = agent
            .run_text_json("system", "user")
            .expect("backup path should succeed");

        assert_eq!(result, r#"{"title":"fallback","paragraphs":["ok"]}"#);
    }

    #[test]
    fn wrong_word_import_ai_output_is_normalized_to_candidates() {
        let parsed = parse_wrong_word_import_ai_output(
            r#"```json
            {
              "warnings": ["messy handwriting"],
              "candidates": [
                {
                  "word": "abandon",
                  "meaning": "放弃",
                  "occurrenceCount": 3,
                  "confidence": 1.4,
                  "evidence": "appears three times"
                },
                {"word": "   "}
              ]
            }
            ```"#,
            "image",
            "photo.jpg",
        )
        .expect("parse import ai output");

        assert_eq!(parsed["sourceType"], "image");
        assert_eq!(parsed["sourceName"], "photo.jpg");
        assert_eq!(parsed["warnings"][0], "messy handwriting");
        assert_eq!(parsed["candidates"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["candidates"][0]["word"], "abandon");
        assert_eq!(parsed["candidates"][0]["candidateId"], "abandon");
        assert_eq!(parsed["candidates"][0]["confidence"], 1.0);
        assert_eq!(parsed["candidates"][0]["isHighFrequency"], true);
    }

    #[test]
    fn wrong_word_image_analysis_uses_backup_after_primary_failure() {
        let primary_url = spawn_json_server("500 Internal Server Error", r#"{"error":"down"}"#);
        let backup_url = spawn_json_server(
            "200 OK",
            r#"{"output":[{"content":[{"type":"output_text","text":"{\"sourceType\":\"image\",\"candidates\":[{\"word\":\"abandon\",\"occurrenceCount\":2,\"confidence\":0.8,\"evidence\":\"seen twice\"}]}"}]}]}"#,
        );
        let _primary_url = EnvGuard::set("ANTHROPIC_BASE_URL", &primary_url);
        let _primary_key = EnvGuard::set("ANTHROPIC_AUTH_TOKEN", "primary-test-key");
        let _backup_url = EnvGuard::set("OPENAI_BASE_URL", &backup_url);
        let _backup_key = EnvGuard::set("OPENAI_AUTH_TOKEN", "backup-test-key");
        let _backup_model = EnvGuard::set("OPENAI_MODEL", "gpt-test");

        let result =
            analyze_wrong_word_image_with_ai("photo.jpg", "image/jpeg", "ZmFrZS1pbWFnZS1ieXRlcw==")
                .expect("backup image analysis should succeed");

        assert!(result.contains("\"word\":\"abandon\""));
    }

    #[test]
    fn today_completion_includes_unfinished_active_session_progress() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let today = "2026-04-28";
        let snapshot = serde_json::json!({
            "questionEngineVersion": 4,
            "session": {
                "sessionId": "sess_partial",
                "mode": "newWord",
                "totalWords": 5,
                "wordbookId": null,
                "startedAt": "2026-04-28T12:00:00Z"
            },
            "questions": [],
            "results": [
                {"questionId": "q1"},
                {"questionId": "q2"},
                {"questionId": "q3"}
            ],
            "currentIndex": 3
        });
        conn.execute(
            "INSERT INTO app_settings (key, value_json) VALUES (?1, ?2)",
            ("active_study_session_newWord", snapshot.to_string()),
        )
        .expect("insert active session snapshot");

        let completions = load_today_completion_seed(&conn, today).expect("load completion seed");

        assert_eq!(completions.new_words_completed, 3);
        assert_eq!(completions.review_words_completed, 0);
    }

    #[test]
    fn stored_session_mode_normalization_accepts_json_or_plain_text() {
        assert_eq!(normalize_stored_session_mode("\"newWord\""), "newWord");
        assert_eq!(normalize_stored_session_mode("review"), "review");
    }

    #[test]
    fn review_today_target_is_zero_without_prior_learning_history() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let plan = serde_json::json!({
            "newWordsPerDay": 5,
            "reviewWordsPerDay": 6,
            "mixedTestPerDay": 4,
            "wrongWordTestPerDay": 3,
            "rootAffixPerDay": 2
        });
        let targets = today_target_seed_from_plan_value(&plan);

        let aligned =
            align_today_targets_to_available_pools(&conn, targets, &plan).expect("align targets");

        assert_eq!(aligned.review_words_target, Some(0));
        assert_eq!(aligned.review_words_base_target, Some(0));
    }

    #[test]
    fn today_target_seed_reflects_saved_plan_when_applied_to_today() {
        let plan = serde_json::json!({
            "newWordsPerDay": 6,
            "reviewWordsPerDay": 6,
            "mixedTestPerDay": 4,
            "wrongWordTestPerDay": 3,
            "rootAffixPerDay": 2,
            "growthRuleStartDate": "2026-04-30",
            "sharedGrowthRule": {
                "intervalDays": 7,
                "increment": 5
            }
        });

        let targets = today_target_seed_from_plan_value_for_date(&plan, "2026-04-30");

        assert_eq!(targets.new_words_target, Some(24));
        assert_eq!(targets.new_words_base_target, Some(24));
    }

    #[test]
    fn applying_plan_to_today_clears_stale_active_session_snapshot() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let old_session = serde_json::json!({
            "questionEngineVersion": 4,
            "session": {
                "sessionId": "sess_old_20",
                "mode": "newWord",
                "totalWords": 5,
                "wordbookId": null,
                "startedAt": "2026-04-30T01:00:00Z"
            },
            "questions": (0..20)
                .map(|index| serde_json::json!({
                    "questionId": format!("q{index}"),
                    "entrySourceId": format!("word{index}"),
                    "word": format!("word{index}"),
                    "prompt": "prompt",
                    "questionType": "enToCnChoice",
                    "choices": [],
                    "correctAnswer": "answer"
                }))
                .collect::<Vec<_>>(),
            "results": [],
            "currentIndex": 3
        });
        set_json_setting(&conn, "active_study_session_newWord", &old_session)
            .expect("seed old active session");

        clear_persisted_active_study_sessions(&conn).expect("clear active sessions");
        let plan = serde_json::json!({
            "newWordsPerDay": 6,
            "reviewWordsPerDay": 0,
            "mixedTestPerDay": 0,
            "wrongWordTestPerDay": 0,
            "rootAffixPerDay": 0,
            "growthRuleStartDate": "2026-04-30",
            "sharedGrowthRule": {
                "intervalDays": 7,
                "increment": 5
            }
        });
        let targets = today_target_seed_from_plan_value_for_date(&plan, "2026-04-30");

        let stale_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM app_settings WHERE key = 'active_study_session_newWord'",
                [],
                |row| row.get(0),
            )
            .expect("count active session snapshots");
        assert_eq!(stale_count, 0);
        assert_eq!(targets.new_words_target, Some(24));
    }

    #[test]
    fn saved_wordbook_change_does_not_change_today_wordbook_until_applied() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        set_json_setting(
            &conn,
            "today_wordbooks_json",
            &serde_json::json!({"1": true}),
        )
        .expect("seed today's wordbook");
        let saved = single_wordbook_selection_json(2, true);
        set_json_setting(&conn, "saved_wordbooks_json", &saved).expect("save future wordbook");

        assert_eq!(
            active_wordbook_ids_from_selection(&saved),
            vec![2],
            "saved selection should be the future wordbook"
        );
        assert_eq!(
            selected_wordbook_ids_for_today(&conn).expect("today wordbooks"),
            vec![1],
            "today should keep the existing wordbook until apply-to-today"
        );
    }

    #[test]
    fn applying_new_wordbook_keeps_today_review_on_old_wordbook_without_candidates() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-review-wordbook-retention', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (1, 'old', 'Old Book', 1, 1, 1),
                    (2, 'new', 'New Book', 1, 1, 1)",
            [],
        )
        .expect("insert wordbooks");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'old_learned', 'alpha', 'n.', 2.0),
                    (2, 1, 'new_unlearned', 'beta', 'n.', 1.0)",
            [],
        )
        .expect("insert entries");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (1, 1, 1), (2, 2, 1)",
            [],
        )
        .expect("insert wordbook entries");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_old_review_seed', '\"newWord\"', 1, '2026-04-29T01:00:00Z', '2026-04-29T01:10:00Z')",
            [],
        )
        .expect("insert study session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_old_review_seed', 'q1', 1, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, '2026-04-29T01:01:00Z')",
            [],
        )
        .expect("insert study result");
        set_json_setting(
            &conn,
            "today_review_wordbooks_json",
            &serde_json::json!({"1": true}),
        )
        .expect("seed old review wordbook");
        let new_selection = serde_json::json!({"2": true});

        assert!(
            !should_replace_today_review_wordbooks(&conn, &new_selection)
                .expect("new wordbook has no review candidates")
        );
        assert_eq!(
            selected_review_wordbook_ids_for_today(&conn).expect("review wordbooks"),
            vec![1]
        );
    }

    #[test]
    fn growth_rule_increases_today_targets_after_elapsed_intervals() {
        let plan = serde_json::json!({
            "newWordsPerDay": 5,
            "reviewWordsPerDay": 6,
            "mixedTestPerDay": 4,
            "wrongWordTestPerDay": 3,
            "rootAffixPerDay": 2,
            "growthRuleMode": "shared",
            "growthRuleStartDate": "2026-04-01",
            "sharedGrowthRule": {
                "intervalDays": 7,
                "increment": 1
            }
        });

        let targets = today_target_seed_from_plan_value_for_date(&plan, "2026-04-15");

        assert_eq!(targets.new_words_target, Some(28));
        assert_eq!(targets.review_words_target, Some(32));
        assert_eq!(targets.mixed_test_target, Some(6));
        assert_eq!(targets.wrong_word_test_target, Some(5));
        assert_eq!(targets.root_affix_target, Some(4));
    }

    #[test]
    fn missing_growth_start_date_does_not_backfill_growth_from_epoch() {
        let plan = serde_json::json!({
            "newWordsPerDay": 5,
            "reviewWordsPerDay": 0,
            "mixedTestPerDay": 0,
            "wrongWordTestPerDay": 0,
            "rootAffixPerDay": 0,
            "sharedGrowthRule": {
                "intervalDays": 7,
                "increment": 5
            }
        });

        let targets = today_target_seed_from_plan_value_for_date(&plan, "2026-04-30");

        assert_eq!(targets.new_words_target, Some(20));
    }

    #[test]
    fn seed_vocab_import_restores_non_root_today_targets() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("word-seed-vocab-test-{nonce}"));
        let book_dir = bundle_dir.join("seed-vocab").join("book");
        fs::create_dir_all(&book_dir).expect("create seed vocab book dir");
        fs::write(
            book_dir.join("CET4_3.json"),
            r#"[
              {"wordRank":1,"headWord":"alpha","content":{"word":{"wordId":"CET4_3_1","content":{"usphone":"a","ukphone":"a","trans":[{"pos":"n","tranCn":"first","tranOther":"first"}],"sentence":{"sentences":[{"sContent":"Alpha starts.","sCn":"Alpha starts."}]}}}}},
              {"wordRank":2,"headWord":"beta","content":{"word":{"wordId":"CET4_3_2","content":{"usphone":"b","ukphone":"b","trans":[{"pos":"n","tranCn":"second","tranOther":"second"}],"sentence":{"sentences":[{"sContent":"Beta follows.","sCn":"Beta follows."}]}}}}}
            ]"#,
        )
        .expect("write seed vocab fixture");

        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        ensure_planning_state(&conn).expect("ensure planning state");
        ensure_seed_vocabulary_imported(&conn, &bundle_dir).expect("import seed vocabulary");

        let plan = serde_json::json!({
            "newWordsPerDay": 2,
            "reviewWordsPerDay": 0,
            "mixedTestPerDay": 2,
            "wrongWordTestPerDay": 0,
            "rootAffixPerDay": 1
        });
        let targets = today_target_seed_from_plan_value(&plan);
        let aligned =
            align_today_targets_to_available_pools(&conn, targets, &plan).expect("align targets");

        assert_eq!(aligned.new_words_target, Some(8));
        assert_eq!(aligned.mixed_test_target, Some(2));
        assert_eq!(aligned.root_affix_target, Some(1));

        fs::remove_dir_all(bundle_dir).expect("cleanup temp bundle dir");
    }

    #[test]
    fn real_seed_assets_hydrate_all_today_study_modes_from_selected_wordbook() {
        let bundle_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/mobile/android/app/src/main/assets");
        assert!(
            bundle_dir.join("seed-vocab/book/KaoYan_3.json").exists(),
            "real KaoYan seed asset should exist"
        );

        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        ensure_planning_state(&conn).expect("ensure planning state");
        ensure_seed_vocabulary_imported(&conn, &bundle_dir).expect("import real seed vocabulary");
        set_json_setting(
            &conn,
            "saved_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select real KaoYan wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 2,
                "reviewWordsPerDay": 1,
                "mixedTestPerDay": 2,
                "wrongWordTestPerDay": 1,
                "rootAffixPerDay": 2
            }),
        )
        .expect("set today plan");

        let mut stmt = conn
            .prepare(
                "SELECT e.id, e.source_entry_key
                 FROM wordbook_entries we
                 JOIN entries e ON e.id = we.entry_id
                 WHERE we.wordbook_id = 3
                 ORDER BY we.rank_in_book ASC, e.id ASC
                 LIMIT 4",
            )
            .expect("prepare real wordbook query");
        let seed_entries = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .expect("query real wordbook entries")
            .collect::<Result<Vec<_>, _>>()
            .expect("decode real wordbook entries");
        assert!(
            seed_entries.len() >= 4,
            "real KaoYan wordbook should import enough entries for all modes"
        );

        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, wordbook_id, started_at, completed_at)
             VALUES ('sess_real_seed_history', '\"newWord\"', 2, 3, '2026-04-29T01:00:00Z', '2026-04-29T01:10:00Z')",
            [],
        )
        .expect("insert study session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_real_seed_history', 'q_correct', ?1, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, '2026-04-29T01:01:00Z'),
                    ('sess_real_seed_history', 'q_wrong', ?2, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-04-29T01:02:00Z')",
            rusqlite::params![seed_entries[0].0, seed_entries[1].0],
        )
        .expect("insert real study results");

        let new_word = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::NewWord,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            Some(bundle_dir.clone()),
        )
        .expect("hydrate new word mode from real seed");
        assert_real_wordbook_payloads("new word", &new_word.entry_payloads);
        assert!(
            !new_word.entry_source_ids.contains(&seed_entries[0].1)
                && !new_word.entry_source_ids.contains(&seed_entries[1].1),
            "new word mode should skip entries already seen in study_results"
        );

        let review = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::Review,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            Some(bundle_dir.clone()),
        )
        .expect("hydrate review mode from real seed");
        assert_real_wordbook_payloads("review", &review.entry_payloads);
        assert_eq!(review.entry_source_ids, vec![seed_entries[0].1.clone()]);

        let mixed = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            Some(bundle_dir.clone()),
        )
        .expect("hydrate mixed mode from real seed");
        assert_real_wordbook_payloads("mixed test", &mixed.entry_payloads);

        let wrong = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::WrongWordReinforcement,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            Some(bundle_dir.clone()),
        )
        .expect("hydrate wrong-word mode from real seed");
        assert_real_wordbook_payloads("wrong-word reinforcement", &wrong.entry_payloads);
        assert_eq!(wrong.entry_source_ids, vec![seed_entries[1].1.clone()]);

        let root_affix = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::RootAffix,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            Some(bundle_dir),
        )
        .expect("hydrate root-affix mode from real seed");
        assert!(!root_affix.entry_payloads.is_empty());
        assert!(root_affix.entry_payloads.iter().all(|payload| {
            payload.source_id.starts_with("root_affix_")
                && !payload.source_id.starts_with("active_session_restore_")
                && !sample_test_words().contains(&payload.word.as_str())
        }));
    }

    fn assert_real_wordbook_payloads(
        label: &str,
        payloads: &[word_storage_core::models::StartSessionEntryPayload],
    ) {
        assert!(
            !payloads.is_empty(),
            "{label} should return real seed payloads"
        );
        assert!(
            payloads.iter().all(|payload| {
                payload.source_id.starts_with("KaoYan_3")
                    && !payload.source_id.starts_with("active_session_restore_")
                    && !sample_test_words().contains(&payload.word.as_str())
                    && !payload.meanings.is_empty()
            }),
            "{label} should use imported KaoYan seed entries, not sample/test entries"
        );
    }

    fn sample_test_words() -> [&'static str; 5] {
        ["adapt", "approach", "remote", "salary", "enhance"]
    }

    #[test]
    fn seed_vocabulary_import_cleans_stray_angle_markers_from_meanings() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("word-seed-vocab-clean-test-{nonce}"));
        let book_dir = bundle_dir.join("seed-vocab").join("book");
        fs::create_dir_all(&book_dir).expect("create seed book dir");
        fs::write(
            book_dir.join("KaoYan_3.json"),
            r#"[{
              "wordRank":1,
              "headWord":"assimilate",
              "content":{"word":{"wordId":"KaoYan_3_600","content":{
                "trans":[{"pos":"v","tranCn":"经消化而吸收 <；同化","tranOther":"to absorb"}]
              }}}
            },{
              "headWord":"rebuild",
              "content":{"word":{"wordHead":"rebuild","content":{
                "trans":[{"tranCn":"é‡å»º"}],
                "remMethod":{"val":"re(é–²å¶†æŸŠ) + build(å»ºé€ ) -> rebuild"}
              }}}
            },{
              "headWord":"rebuild",
              "content":{"word":{"wordHead":"rebuild","content":{
                "trans":[{"tranCn":"\u91cd\u5efa"}],
                "remMethod":{"val":"re(\u518d) + build(\u5efa\u9020) -> rebuild"}
              }}}
            }]"#,
        )
        .expect("write seed book fixture");

        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");

        ensure_seed_vocabulary_imported(&conn, &bundle_dir).expect("import seed vocabulary");

        let meaning: String = conn
            .query_row("SELECT meaning_cn FROM entry_meanings", [], |row| {
                row.get(0)
            })
            .expect("load imported meaning");
        assert_eq!(meaning, "经消化而吸收；同化");

        fs::remove_dir_all(bundle_dir).expect("cleanup temp bundle dir");
    }

    #[test]
    fn seed_vocabulary_repair_cleans_existing_stray_angle_markers() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit) VALUES (1, 'test')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma)
             VALUES (1, 1, 'KaoYan_3_600', 'assimilate', 'assimilate')",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'v', '经消化而吸收 <；同化', 0)",
            [],
        )
        .expect("insert noisy meaning");

        repair_seed_vocabulary_dedup(&conn).expect("repair seed vocabulary");

        let meaning: String = conn
            .query_row("SELECT meaning_cn FROM entry_meanings", [], |row| {
                row.get(0)
            })
            .expect("load repaired meaning");
        assert_eq!(meaning, "经消化而吸收；同化");
    }

    #[test]
    fn seed_vocabulary_repair_cleans_kaoyan_exam_markers() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit) VALUES (1, 'test')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma)
             VALUES (1, 1, 'KaoYan_3_820', 'foundation', 'foundation')",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'n', '基础， 根本；建立， 创立；地基；基金， 基金会 A', 0)",
            [],
        )
        .expect("insert noisy meaning");

        repair_seed_vocabulary_dedup(&conn).expect("repair seed vocabulary");

        let meaning: String = conn
            .query_row("SELECT meaning_cn FROM entry_meanings", [], |row| {
                row.get(0)
            })
            .expect("load repaired meaning");
        assert_eq!(meaning, "基础，根本；建立，创立；地基；基金，基金会");
    }

    #[test]
    fn seed_vocabulary_repair_cleans_persisted_ai_and_session_json() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        set_json_setting(
            &conn,
            "ai_passage_history_json",
            &serde_json::json!([{
                "passageId": "p1",
                "blocks": [{
                    "blockType": "paragraph",
                    "segments": [{
                        "type": "word",
                        "text": "assimilate",
                        "glossZh": "经消化而吸收 <；同化"
                    }]
                }]
            }]),
        )
        .expect("insert ai history");
        set_json_setting(
            &conn,
            "active_study_session_newWord",
            &serde_json::json!({
                "questions": [{
                    "acceptedMeanings": ["经消化而吸收 <；同化"],
                    "choices": [{"label": "A", "text": "经消化而吸收 <；同化"}]
                }]
            }),
        )
        .expect("insert active session");

        repair_seed_vocabulary_dedup(&conn).expect("repair seed vocabulary");

        let ai_history = get_json_setting(&conn, "ai_passage_history_json", &serde_json::json!([]))
            .expect("load ai history");
        let session = get_json_setting(
            &conn,
            "active_study_session_newWord",
            &serde_json::json!({}),
        )
        .expect("load active session");
        assert_eq!(
            ai_history.pointer("/0/blocks/0/segments/0/glossZh"),
            Some(&serde_json::json!("经消化而吸收；同化"))
        );
        assert_eq!(
            session.pointer("/questions/0/choices/0/text"),
            Some(&serde_json::json!("经消化而吸收；同化"))
        );
    }

    #[test]
    fn review_session_does_not_use_unlearned_general_word_pool() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-review-no-fallback', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'entry_one', 'alpha', 'n.', 1.0)",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'n.', 'alpha meaning', 0)",
            [],
        )
        .expect("insert meaning");
        let request = word_storage_core::models::StartSessionRequest {
            mode: word_storage_core::models::SessionMode::Review,
            wordbook_id: None,
            entry_source_ids: Vec::new(),
            entry_payloads: Vec::new(),
            distractor_payloads: Vec::new(),
        };

        let result = hydrate_start_session_request(&conn, request, None);

        assert!(result.is_err());
    }

    #[test]
    fn review_uses_learned_entries_and_mixed_uses_current_wordbook_entries() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-learned-current-book', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 2, 1),
                    (4, 'Other', 'Other', 1, 1, 0)",
            [],
        )
        .expect("insert wordbooks");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'learned_current', 'alpha', 'n.', 3.0),
                    (2, 1, 'unlearned_current', 'beta', 'n.', 2.0),
                    (3, 1, 'learned_other', 'gamma', 'n.', 1.0)",
            [],
        )
        .expect("insert entries");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'n.', 'alpha meaning', 0),
                    (2, 'n.', 'beta meaning', 0),
                    (3, 'n.', 'gamma meaning', 0)",
            [],
        )
        .expect("insert meanings");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1), (3, 2, 2), (4, 3, 1)",
            [],
        )
        .expect("insert wordbook entries");
        set_json_setting(
            &conn,
            "saved_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select saved wordbook");
        set_json_setting(
            &conn,
            "today_wordbooks_json",
            &serde_json::json!({"4": true}),
        )
        .expect("seed stale today wordbook");
        set_json_setting(
            &conn,
            "today_review_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("preserve review wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 0,
                "reviewWordsPerDay": 2,
                "mixedTestPerDay": 2,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 0
            }),
        )
        .expect("set plan");
        let prior_day = (chrono::Local::now().date_naive() - chrono::Duration::days(2))
            .format("%Y-%m-%d")
            .to_string();
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_learned', '\"newWord\"', 2, ?1, ?2)",
            [
                format!("{prior_day}T01:00:00Z"),
                format!("{prior_day}T01:10:00Z"),
            ],
        )
        .expect("insert session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_learned', 'q1', 1, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, ?1),
                    ('sess_learned', 'q2', 3, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, ?2)",
            [
                format!("{prior_day}T01:01:00Z"),
                format!("{prior_day}T01:02:00Z"),
            ],
        )
        .expect("insert results");

        let today = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_today', '\"newWord\"', 1, ?1, ?2)",
            [format!("{today}T01:00:00Z"), format!("{today}T01:10:00Z")],
        )
        .expect("insert today session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_today', 'q_today', 2, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, ?1)",
            [format!("{today}T01:01:00Z")],
        )
        .expect("insert today result");

        let review = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::Review,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            None,
        )
        .expect("hydrate review mode");
        assert_eq!(review.entry_source_ids, vec!["learned_current"]);

        let mixed = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            None,
        )
        .expect("hydrate mixed mode");
        let mut mixed_ids = mixed.entry_source_ids;
        mixed_ids.sort();
        assert_eq!(mixed_ids, vec!["learned_other"]);
    }

    #[test]
    fn review_targets_include_incorrect_prior_answers_as_learned_words() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-review-incorrect-learned', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 7, 1)",
            [],
        )
        .expect("insert wordbook");
        for id in 1..=7 {
            conn.execute(
                "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
                 VALUES (?1, 1, ?2, ?3, 'n.', ?4)",
                rusqlite::params![
                    id,
                    format!("review_entry_{id}"),
                    format!("word{id}"),
                    100.0 - id as f64
                ],
            )
            .expect("insert entry");
            conn.execute(
                "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
                 VALUES (?1, 'n.', ?2, 0)",
                rusqlite::params![id, format!("meaning {id}")],
            )
            .expect("insert meaning");
            conn.execute(
                "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
                 VALUES (3, ?1, ?1)",
                [id],
            )
            .expect("insert wordbook entry");
        }
        set_json_setting(
            &conn,
            "today_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("set today wordbook");
        set_json_setting(
            &conn,
            "today_review_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("set today review wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 0,
                "reviewWordsPerDay": 7,
                "mixedTestPerDay": 0,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 0
            }),
        )
        .expect("set today plan");
        let prior_day = (chrono::Local::now().date_naive() - chrono::Duration::days(2))
            .format("%Y-%m-%d")
            .to_string();
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_review_pool', '\"newWord\"', 7, ?1, ?2)",
            [
                format!("{prior_day}T01:00:00Z"),
                format!("{prior_day}T01:10:00Z"),
            ],
        )
        .expect("insert study session");
        for id in 1..=7 {
            let outcome = if id <= 5 {
                "\"correct\""
            } else {
                "\"incorrect\""
            };
            conn.execute(
                "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
                 correct_answer, outcome, response_time_ms, answered_at)
                 VALUES ('sess_review_pool', ?1, ?2, '\"enToCnChoice\"', 'A', 'B', ?3, 100, ?4)",
                rusqlite::params![
                    format!("q{id}"),
                    id,
                    outcome,
                    format!("{prior_day}T01:{id:02}:00Z")
                ],
            )
            .expect("insert study result");
        }

        let learned_count = load_learned_entry_ids_for_wordbooks(&conn, &[3], 7)
            .expect("load learned entries")
            .len();
        assert_eq!(learned_count, 7);

        let targets = today_target_seed_from_plan_value_for_date(
            &serde_json::json!({"reviewWordsPerDay": 7}),
            &today_date_string(),
        );
        let aligned = align_today_targets_to_available_pools(
            &conn,
            targets,
            &serde_json::json!({"reviewWordsPerDay": 7}),
        )
        .expect("align today targets");

        assert_eq!(aligned.review_words_target, Some(28));
    }

    #[test]
    fn review_wordbook_defaults_to_today_wordbook_when_review_snapshot_is_missing() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        set_json_setting(
            &conn,
            "today_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("set today wordbook");

        assert_eq!(
            selected_review_wordbook_ids_for_today(&conn).expect("review wordbooks"),
            vec![3]
        );
    }

    #[test]
    fn cloud_restore_wordbooks_refreshes_today_review_wordbook_selection() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        set_json_setting(
            &conn,
            "today_review_wordbooks_json",
            &serde_json::json!({"1": true}),
        )
        .expect("seed stale review wordbook");

        reset_user_owned_local_data(&conn).expect("reset local user data");
        restore_cloud_wordbook_preferences(
            &conn,
            Some(&serde_json::json!([
                {"wordbook_id": 3, "is_active": true},
                {"wordbook_id": 4, "is_active": false}
            ])),
        )
        .expect("restore cloud wordbook preferences");

        assert_eq!(
            selected_review_wordbook_ids_for_today(&conn).expect("review wordbooks"),
            vec![3]
        );
    }

    #[test]
    fn review_target_falls_back_to_today_wordbook_when_review_snapshot_has_no_candidates() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-review-fallback-today-wordbook', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (1, 'CET4', 'CET-4', 1, 1, 1),
                    (3, 'KaoYan', 'KaoYan', 1, 1, 1)",
            [],
        )
        .expect("insert wordbooks");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (10, 1, 'today_learned', 'alpha', 'n.', 1.0),
                    (20, 1, 'stale_empty', 'beta', 'n.', 1.0)",
            [],
        )
        .expect("insert entries");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (10, 'n.', 'alpha meaning', 0),
                    (20, 'n.', 'beta meaning', 0)",
            [],
        )
        .expect("insert meanings");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 10, 1), (1, 20, 1)",
            [],
        )
        .expect("insert wordbook entries");
        set_json_setting(
            &conn,
            "today_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("set today wordbook");
        set_json_setting(
            &conn,
            "today_review_wordbooks_json",
            &serde_json::json!({"1": true}),
        )
        .expect("set stale review wordbook");
        let prior_day = (chrono::Local::now().date_naive() - chrono::Duration::days(2))
            .format("%Y-%m-%d")
            .to_string();
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_today_wordbook_learned', '\"newWord\"', 1, ?1, ?2)",
            [
                format!("{prior_day}T01:00:00Z"),
                format!("{prior_day}T01:10:00Z"),
            ],
        )
        .expect("insert study session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_today_wordbook_learned', 'q1', 10, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, ?1)",
            [format!("{prior_day}T01:01:00Z")],
        )
        .expect("insert study result");

        assert_eq!(
            selected_review_wordbook_ids_for_today(&conn).expect("review wordbooks"),
            vec![3]
        );
        let targets = today_target_seed_from_plan_value_for_date(
            &serde_json::json!({"reviewWordsPerDay": 1}),
            &today_date_string(),
        );
        let aligned = align_today_targets_to_available_pools(
            &conn,
            targets,
            &serde_json::json!({"reviewWordsPerDay": 1}),
        )
        .expect("align today targets");
        assert_eq!(aligned.review_words_target, Some(4));
    }

    #[test]
    fn review_target_falls_back_to_global_prior_learning_when_selected_books_are_empty() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-review-global-fallback', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'ACTIVE_EMPTY', 'Active Empty', 1, 1, 1),
                    (4, 'OLD_LEARNED', 'Old Learned', 1, 1, 1)",
            [],
        )
        .expect("insert wordbooks");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (10, 1, 'active_empty', 'empty', 'n.', 1.0),
                    (20, 1, 'old_learned', 'learned', 'n.', 1.0)",
            [],
        )
        .expect("insert entries");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (10, 'n.', 'empty meaning', 0),
                    (20, 'n.', 'learned meaning', 0)",
            [],
        )
        .expect("insert meanings");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 10, 1), (4, 20, 1)",
            [],
        )
        .expect("insert wordbook entries");
        set_json_setting(
            &conn,
            "today_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("set today wordbook");
        set_json_setting(
            &conn,
            "today_review_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("set review wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 0,
                "reviewWordsPerDay": 1,
                "mixedTestPerDay": 0,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 0
            }),
        )
        .expect("set today plan");
        let prior_day = (chrono::Local::now().date_naive() - chrono::Duration::days(2))
            .format("%Y-%m-%d")
            .to_string();
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_global_prior', '\"newWord\"', 1, ?1, ?2)",
            [
                format!("{prior_day}T01:00:00Z"),
                format!("{prior_day}T01:10:00Z"),
            ],
        )
        .expect("insert study session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_global_prior', 'q1', 20, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, ?1)",
            [format!("{prior_day}T01:01:00Z")],
        )
        .expect("insert study result");

        assert!(load_learned_entry_ids_for_wordbooks(&conn, &[3], 1)
            .expect("selected review pool")
            .is_empty());
        assert_eq!(
            load_review_entry_ids_for_today(&conn, &[3], 1).expect("global review fallback"),
            vec![20]
        );

        let targets = today_target_seed_from_plan_value_for_date(
            &serde_json::json!({"reviewWordsPerDay": 1}),
            &today_date_string(),
        );
        let aligned = align_today_targets_to_available_pools(
            &conn,
            targets,
            &serde_json::json!({"reviewWordsPerDay": 1}),
        )
        .expect("align today targets");
        assert_eq!(aligned.review_words_target, Some(4));

        let hydrated = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::Review,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            None,
        )
        .expect("hydrate review");
        assert_eq!(hydrated.entry_source_ids, vec!["old_learned".to_string()]);
    }

    #[test]
    fn review_selection_spreads_across_prior_learning_age_buckets() {
        let candidates = vec![
            ReviewCandidate {
                entry_id: 1,
                first_learned_at: "2026-04-30T01:00:00+08:00".to_string(),
                last_answered_at: "2026-04-30T01:00:00+08:00".to_string(),
                frequency: 1.0,
            },
            ReviewCandidate {
                entry_id: 2,
                first_learned_at: "2026-04-29T01:00:00+08:00".to_string(),
                last_answered_at: "2026-04-29T01:00:00+08:00".to_string(),
                frequency: 1.0,
            },
            ReviewCandidate {
                entry_id: 3,
                first_learned_at: "2026-04-26T01:00:00+08:00".to_string(),
                last_answered_at: "2026-04-26T01:00:00+08:00".to_string(),
                frequency: 1.0,
            },
            ReviewCandidate {
                entry_id: 4,
                first_learned_at: "2026-04-20T01:00:00+08:00".to_string(),
                last_answered_at: "2026-04-20T01:00:00+08:00".to_string(),
                frequency: 1.0,
            },
            ReviewCandidate {
                entry_id: 5,
                first_learned_at: "2026-04-01T01:00:00+08:00".to_string(),
                last_answered_at: "2026-04-01T01:00:00+08:00".to_string(),
                frequency: 1.0,
            },
            ReviewCandidate {
                entry_id: 6,
                first_learned_at: "2026-05-01T01:00:00+08:00".to_string(),
                last_answered_at: "2026-05-01T01:00:00+08:00".to_string(),
                frequency: 1.0,
            },
        ];

        let selected = select_review_candidates(candidates, "2026-05-01", 4);

        assert!(!selected.contains(&6), "today's new word must be excluded");
        assert!(
            selected.contains(&1),
            "recent prior words should be represented"
        );
        assert!(
            selected.iter().any(|id| *id >= 3),
            "older buckets should be represented"
        );
        assert_eq!(selected.len(), 4);
    }
    #[test]
    fn new_word_hydration_skips_words_already_seen_in_study_results() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-new-word-unlearned', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 3, 1)",
            [],
        )
        .expect("insert wordbook");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'already_learned', 'alpha', 'n.', 3.0),
                    (2, 1, 'fresh_one', 'beta', 'n.', 2.0),
                    (3, 1, 'fresh_two', 'gamma', 'n.', 1.0)",
            [],
        )
        .expect("insert entries");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'n.', 'alpha meaning', 0),
                    (2, 'n.', 'beta meaning', 0),
                    (3, 'n.', 'gamma meaning', 0)",
            [],
        )
        .expect("insert meanings");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1), (3, 2, 2), (3, 3, 3)",
            [],
        )
        .expect("insert wordbook entries");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_seen', '\"newWord\"', 1, '2026-04-29T01:00:00Z', '2026-04-29T01:10:00Z')",
            [],
        )
        .expect("insert study session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_seen', 'q1', 1, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, '2026-04-29T01:01:00Z')",
            [],
        )
        .expect("insert study result");
        set_json_setting(
            &conn,
            "saved_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select active wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 2,
                "reviewWordsPerDay": 0,
                "mixedTestPerDay": 0,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 0
            }),
        )
        .expect("set today plan");

        let hydrated = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::NewWord,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            None,
        )
        .expect("hydrate new word mode");

        let mut source_ids = hydrated.entry_source_ids;
        source_ids.sort();
        assert_eq!(source_ids, vec!["fresh_one", "fresh_two"]);
    }

    #[test]
    fn new_word_hydration_uses_daily_stable_order_instead_of_rank_prefix() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-daily-new-word-order', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 20, 1)",
            [],
        )
        .expect("insert wordbook");
        for rank in 1..=20 {
            conn.execute(
                "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
                 VALUES (?1, 1, ?2, ?3, 'n.', ?4)",
                rusqlite::params![
                    rank,
                    format!("entry_{rank}"),
                    format!("word{rank}"),
                    1_000.0 - rank as f64
                ],
            )
            .expect("insert entry");
            conn.execute(
                "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
                 VALUES (?1, 'n.', ?2, 0)",
                rusqlite::params![rank, format!("meaning {rank}")],
            )
            .expect("insert meaning");
            conn.execute(
                "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
                 VALUES (3, ?1, ?1)",
                [rank],
            )
            .expect("insert wordbook entry");
        }

        let today =
            load_unlearned_ranked_entry_ids_for_wordbooks_on_date(&conn, &[3], 4, "2026-04-30")
                .expect("load today's new words");
        let today_again =
            load_unlearned_ranked_entry_ids_for_wordbooks_on_date(&conn, &[3], 4, "2026-04-30")
                .expect("load today's new words again");
        let tomorrow =
            load_unlearned_ranked_entry_ids_for_wordbooks_on_date(&conn, &[3], 4, "2026-05-01")
                .expect("load tomorrow's new words");

        assert_eq!(today, today_again);
        assert_ne!(today, vec![1, 2, 3, 4]);
        assert_ne!(today, tomorrow);
    }

    #[test]
    fn new_word_hydration_does_not_fallback_to_global_pool_when_wordbook_is_empty() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-global-fallback-guard', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 0, 1)",
            [],
        )
        .expect("insert selected empty wordbook");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'test_pool_entry', 'adapt', 'v.', 9.9)",
            [],
        )
        .expect("insert stale global entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'v.', '适应', 0)",
            [],
        )
        .expect("insert stale global meaning");
        set_json_setting(
            &conn,
            "saved_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select empty wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 1,
                "reviewWordsPerDay": 0,
                "mixedTestPerDay": 0,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 0
            }),
        )
        .expect("set today plan");

        let result = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::NewWord,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            None,
        );

        assert!(result
            .expect_err("empty selected wordbook must not use global stale entries")
            .contains("No study entries available"),);
    }

    #[test]
    fn new_word_session_uses_growth_adjusted_today_target_count() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-growth-adjusted-session-target', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 12, 1)",
            [],
        )
        .expect("insert wordbook");
        for rank in 1..=12 {
            conn.execute(
                "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
                 VALUES (?1, 1, ?2, ?3, 'n.', ?4)",
                rusqlite::params![
                    rank,
                    format!("growth_entry_{rank}"),
                    format!("growthword{rank}"),
                    1_000.0 - rank as f64
                ],
            )
            .expect("insert entry");
            conn.execute(
                "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
                 VALUES (?1, 'n.', ?2, 0)",
                rusqlite::params![rank, format!("growth meaning {rank}")],
            )
            .expect("insert meaning");
            conn.execute(
                "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
                 VALUES (3, ?1, ?1)",
                [rank],
            )
            .expect("insert wordbook entry");
        }
        set_json_setting(
            &conn,
            "saved_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select active wordbook");
        let today = today_date_string();
        let start_date = (chrono::Local::now().date_naive() - chrono::Days::new(7))
            .format("%Y-%m-%d")
            .to_string();
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 6,
                "reviewWordsPerDay": 0,
                "mixedTestPerDay": 0,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 0,
                "growthRuleMode": "shared",
                "growthRuleStartDate": start_date,
                "sharedGrowthRule": {
                    "intervalDays": 7,
                    "increment": 2
                }
            }),
        )
        .expect("set growth-adjusted today plan");

        let hydrated = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::NewWord,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            None,
        )
        .expect("hydrate new word session");
        let plan = get_json_setting(&conn, "today_plan_json", &serde_json::json!({}))
            .expect("load today plan");
        let targets = today_target_seed_from_plan_value_for_date(&plan, &today);

        assert_eq!(targets.new_words_target, Some(32));
        assert_eq!(hydrated.entry_payloads.len(), 8);
    }

    #[test]
    fn wrong_answers_are_visible_before_session_completion() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        word_app_core::clear_all_active_sessions();
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-wrong-progress', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 1, 1)",
            [],
        )
        .expect("insert wordbook");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'wrong_entry', 'alpha', 'n.', 1.0)",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'n.', 'alpha meaning', 0)",
            [],
        )
        .expect("insert meaning");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1)",
            [],
        )
        .expect("insert wordbook entry");

        let start = word_app_core::start_study_session(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::MixedTest,
                wordbook_id: Some(3),
                entry_source_ids: vec!["wrong_entry".to_string()],
                entry_payloads: vec![word_storage_core::models::StartSessionEntryPayload {
                    source_id: "wrong_entry".to_string(),
                    word: "alpha".to_string(),
                    part_of_speech: Some("n.".to_string()),
                    frequency: 1.0,
                    phonetic_us: None,
                    phonetic_uk: None,
                    meaning_details: vec![word_storage_core::models::StartSessionMeaningPayload {
                        pos: "n.".to_string(),
                        meaning_cn: "alpha meaning".to_string(),
                        meaning_en: None,
                    }],
                    meanings: vec!["alpha meaning".to_string()],
                    example_sentence: None,
                    example_translation: None,
                }],
                distractor_payloads: Vec::new(),
            },
        )
        .expect("start session");

        word_app_core::submit_study_answer(
            &conn,
            word_storage_core::models::SubmitAnswerRequest {
                question_id: start.current_question.question_id,
                response: "Z".to_string(),
                response_time_ms: 100,
            },
        )
        .expect("submit wrong answer");

        let wrong_words = load_wrong_word_entries(&conn).expect("load wrong words");
        assert_eq!(wrong_words.len(), 1);
        assert_eq!(wrong_words[0]["word"], "alpha");
        assert_eq!(wrong_words[0]["errorCount"], 1);
    }

    #[test]
    fn wrong_word_sync_snapshot_includes_saved_hint() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-hint-sync', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'hint_sync_word', 'numerous', 'adj.', 10.0)",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_hint_sync', '\"mixedTest\"', 1, '2026-05-05T01:00:00Z', NULL)",
            [],
        )
        .expect("insert session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_hint_sync', 'q_hint_sync', 1, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-05T01:01:00Z')",
            [],
        )
        .expect("insert wrong result");
        word_storage_core::persistence::word_hint_repo::save_hint(
            &conn,
            1,
            "拆解：numerous = numer(数)+ous(...的)，记众多的",
            "aiSuggestion",
        )
        .expect("save hint");

        let payload = build_wrong_word_entries_payload(&conn)
            .expect("build wrong-word sync payload")
            .expect("payload exists");
        let entry = &payload["entries"][0];

        assert_eq!(entry["entryId"], 1);
        assert_eq!(
            entry["hintText"],
            "拆解：numerous = numer(数)+ous(...的)，记众多的"
        );
        assert_eq!(entry["hintSource"], "aiSuggestion");
        assert!(entry["hintUpdatedAt"]
            .as_str()
            .map(|value| !value.is_empty())
            .unwrap_or(false));

        word_storage_core::persistence::word_hint_repo::clear_hint(&conn, 1).expect("clear hint");
        let cleared_payload = build_wrong_word_entries_payload(&conn)
            .expect("build cleared wrong-word sync payload")
            .expect("payload exists");
        let cleared_entry = &cleared_payload["entries"][0];
        assert_eq!(cleared_entry["hintText"], "");
        assert_eq!(cleared_entry["hintSource"], "");
        assert_eq!(cleared_entry["hintUpdatedAt"], "");
    }

    #[test]
    fn cloud_wrong_word_restore_imports_hints() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-cloud-hint-restore', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'cloud_hint_word', 'numerous', 'adj.', 10.0)",
            [],
        )
        .expect("insert entry");

        let restored = restore_cloud_wrong_word_hints(
            &conn,
            Some(&serde_json::json!([
                {
                    "entry_id": 1,
                    "hint_text": "拆解：numerous = numer(数)+ous(...的)，记众多的",
                    "hint_source": "aiSuggestion"
                }
            ])),
        )
        .expect("restore cloud hints");
        let hint = word_storage_core::persistence::word_hint_repo::get_hint(&conn, 1)
            .expect("load hint")
            .expect("hint exists");

        assert_eq!(restored, 1);
        assert_eq!(
            hint.hint_text,
            "拆解：numerous = numer(数)+ous(...的)，记众多的"
        );
        assert_eq!(hint.source, "aiSuggestion");
    }

    #[test]
    fn imported_wrong_words_are_persisted_and_visible_in_wrong_word_list() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-imported-wrong-words', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'known_abandon', 'abandon', 'v.', 10.0)",
            [],
        )
        .expect("insert known entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'v.', 'give up', 0)",
            [],
        )
        .expect("insert meaning");

        let request = serde_json::json!({
            "batchId": "batch-import-1",
            "sourceType": "text",
            "sourceName": "external app",
            "acceptedCandidateIds": ["abandon", "unknownword"],
            "candidates": [
                {
                    "candidateId": "abandon",
                    "word": "abandon",
                    "meaning": "give up",
                    "occurrenceCount": 4,
                    "confidence": 0.91,
                    "isHighFrequency": true,
                    "evidence": "appears 4 times"
                },
                {
                    "candidateId": "unknownword",
                    "word": "unknownword",
                    "meaning": "manual note",
                    "occurrenceCount": 1,
                    "confidence": 0.7,
                    "evidence": "appears once"
                }
            ]
        });

        let result = commit_wrong_word_import_with_connection(&conn, &request)
            .expect("commit imported wrong words");
        assert_eq!(result["persisted"], true);
        assert_eq!(result["added"].as_array().unwrap().len(), 2);
        assert_eq!(result["highFrequency"][0]["word"], "abandon");

        let wrong_words = load_wrong_word_entries(&conn).expect("load wrong words");
        assert!(wrong_words
            .iter()
            .any(|word| word["word"] == "abandon" && word["entryId"] == 1));
        let unknown = wrong_words
            .iter()
            .find(|word| word["word"] == "unknownword")
            .expect("unknown import should be visible");
        assert!(unknown["entryId"].as_i64().unwrap() < 0);

        let detail = load_wrong_word_detail_payload_with_bundle(
            &conn,
            unknown["entryId"].as_i64().unwrap(),
            None,
        )
        .expect("load unknown import detail");
        assert_eq!(detail["word"], "unknownword");
        assert_eq!(
            detail["riskBreakdown"][0]["questionType"],
            "importedWrongWord"
        );
    }

    #[test]
    fn wrong_word_payloads_include_saved_user_hint() {
        let conn = Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        let entry_id = seed_basic_entry(&conn, "federal", "联邦的");
        conn.execute(
            "INSERT INTO study_results
             (session_id, question_id, entry_id, question_type, user_response,
              correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('s1', 'q1', ?1, 'enToCnChoice', '', '', 'incorrect', 1, '2026-05-04T00:00:00Z')",
            [entry_id],
        )
        .expect("insert result");
        word_storage_core::persistence::word_hint_repo::save_hint(
            &conn,
            entry_id,
            "fed 联想到 federation，记作联邦的",
            "user",
        )
        .expect("save hint");

        let mut entries = load_wrong_word_entries(&conn).expect("load wrong words");
        assert_eq!(entries.len(), 1);
        let entry = entries.pop().expect("entry");
        assert_eq!(entry["hasHint"], true);
        assert_eq!(entry["userHint"], "fed 联想到 federation，记作联邦的");

        let detail =
            load_wrong_word_detail_payload_with_bundle(&conn, entry_id, None).expect("detail");
        assert_eq!(detail["errorCount"], 1);
        assert_eq!(detail["hasHint"], true);
        assert_eq!(detail["userHint"], "fed 联想到 federation，记作联邦的");
        assert!(detail["hintSuggestions"]
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn hint_recommendation_library_covers_bundled_seed_words() {
        let index: serde_json::Value =
            serde_json::from_str(include_str!("../resources/wordbook_vocabulary_index.json"))
                .expect("vocabulary index json");
        let words = index["words"].as_array().expect("vocabulary words array");
        assert_eq!(words.len(), 7232);

        let mut missing = Vec::new();
        for entry in words {
            let word = entry["word"].as_str().expect("vocabulary word");
            if recommendation_library_for_word(word, &[]).is_empty() {
                missing.push(word.to_string());
            }
        }
        assert!(
            missing.is_empty(),
            "missing recommendations for bundled seed words: {:?}",
            &missing[..missing.len().min(10)]
        );
    }

    #[test]
    fn hint_recommendation_library_has_valid_item_shape() {
        let items: Vec<serde_json::Value> =
            serde_json::from_str(WORD_HINT_RECOMMENDATIONS_JSON).expect("recommendations json");
        assert_eq!(items.len(), 7239);

        let mut ids = std::collections::BTreeSet::new();
        let mut word_styles = std::collections::BTreeSet::new();
        for item in items {
            let id = item["id"].as_str().expect("id");
            let word = item["word"].as_str().expect("word");
            let style = item["style"].as_str().expect("style");
            assert!(!item["label"].as_str().unwrap_or_default().is_empty());
            assert!(!item["text"].as_str().unwrap_or_default().is_empty());
            assert!(!item["wordbookCodes"]
                .as_array()
                .unwrap_or(&Vec::new())
                .is_empty());
            assert!(
                ids.insert(id.to_string()),
                "duplicate recommendation id: {id}"
            );
            assert!(
                word_styles.insert(format!("{word}:{style}")),
                "duplicate recommendation word/style: {word}:{style}"
            );
        }
    }

    #[test]
    fn hint_recommendation_library_uses_runtime_wordbook_codes() {
        let suggestions =
            recommendation_library_for_word("cancel", &["cet4".to_string(), "kaoyan".to_string()]);
        assert!(suggestions.len() >= 3);
        assert_eq!(suggestions[0]["word"], "cancel");
        assert_eq!(
            suggestions[0]["wordbookCodes"],
            serde_json::json!(["cet4", "kaoyan"])
        );
    }

    #[test]
    fn hint_prompt_triggers_after_fifth_skipped_or_incorrect_answer() {
        let conn = Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        let entry_id = seed_basic_entry(&conn, "federal", "联邦的");
        for index in 0..5 {
            let outcome = if index == 4 { "skipped" } else { "incorrect" };
            conn.execute(
                "INSERT INTO study_results
                 (session_id, question_id, entry_id, question_type, user_response,
                  correct_answer, outcome, response_time_ms, answered_at)
                 VALUES ('s1', ?1, ?2, 'enToCnChoice', '', '', ?3, 1, '2026-05-04T00:00:00Z')",
                rusqlite::params![format!("q{index}"), entry_id, outcome],
            )
            .expect("insert result");
        }
        let result = serde_json::json!({
            "entrySourceId": entry_id.to_string(),
            "outcome": "skipped"
        });

        let prompt =
            build_hint_prompt_payload(&conn, &result, &serde_json::Value::Null).expect("prompt");
        assert_eq!(prompt["entryId"], entry_id);
        assert_eq!(prompt["errorCount"], 5);
        assert_eq!(prompt["triggerOutcome"], "skipped");
        assert!(prompt["suggestions"].as_array().unwrap().len() >= 2);

        word_storage_core::persistence::word_hint_repo::save_hint(
            &conn,
            entry_id,
            "already has hint",
            "user",
        )
        .expect("save hint");
        assert!(
            build_hint_prompt_payload(&conn, &result, &serde_json::Value::Null)
                .expect("prompt hidden")
                .is_null()
        );
    }

    #[test]
    fn hint_prompt_can_fall_back_to_current_question_entry_id() {
        let conn = Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        let entry_id = seed_basic_entry(&conn, "cancel", "取消");
        for index in 0..5 {
            conn.execute(
                "INSERT INTO study_results
                 (session_id, question_id, entry_id, question_type, user_response,
                  correct_answer, outcome, response_time_ms, answered_at)
                 VALUES ('s1', ?1, ?2, 'enToCnChoice', '', '', 'incorrect', 1, '2026-05-04T00:00:00Z')",
                rusqlite::params![format!("fallback_q{index}"), entry_id],
            )
            .expect("insert result");
        }
        let result = serde_json::json!({
            "outcome": "incorrect"
        });
        let current_question = serde_json::json!({
            "entrySourceId": entry_id.to_string()
        });

        let prompt = build_hint_prompt_payload(&conn, &result, &current_question).expect("prompt");
        assert_eq!(prompt["entryId"], entry_id);
        assert_eq!(prompt["word"], "cancel");
    }

    #[test]
    fn submit_hint_enrichment_preserves_null_current_question_on_completion() {
        let conn = Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        let mut payload = serde_json::json!({
            "result": {
                "entrySourceId": "1",
                "outcome": "correct"
            },
            "isComplete": true,
            "currentQuestion": null,
            "summary": {
                "sessionId": "s1",
                "totalQuestions": 1,
                "correctCount": 1,
                "fuzzyCorrectCount": 0,
                "incorrectCount": 0,
                "skippedCount": 0,
                "totalWords": 1,
                "wrongWordCount": 0,
                "accuracyPercent": 100,
                "totalTimeMs": 100,
                "completedAt": "2026-05-05T00:00:00Z"
            },
            "nextAction": "Return to today",
            "progress": {"current": 1, "total": 1}
        });

        enrich_submit_response_hints(&conn, &mut payload).expect("enrich submit response");

        assert!(
            payload["currentQuestion"].is_null(),
            "completed submit responses must keep currentQuestion null"
        );
        assert!(payload.get("hintPrompt").is_some());
    }

    #[test]
    fn imported_wrong_word_commit_is_idempotent_by_batch_and_candidate() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let request = serde_json::json!({
            "batchId": "same-batch",
            "sourceType": "image",
            "sourceName": "photo",
            "acceptedCandidateIds": ["repeat"],
            "candidates": [{
                "candidateId": "repeat",
                "word": "repeat",
                "occurrenceCount": 3,
                "confidence": 0.8,
                "isHighFrequency": true,
                "evidence": "seen repeatedly"
            }]
        });

        let first =
            commit_wrong_word_import_with_connection(&conn, &request).expect("first import commit");
        let second = commit_wrong_word_import_with_connection(&conn, &request)
            .expect("second import commit");

        assert_eq!(first["added"].as_array().unwrap().len(), 1);
        assert_eq!(second["added"].as_array().unwrap().len(), 0);
        assert_eq!(second["skipped"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn wrong_word_reinforcement_uses_only_imports_mapped_to_entries() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-imported-reinforcement', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'known_abandon', 'abandon', 'v.', 10.0)",
            [],
        )
        .expect("insert known entry");
        let request = serde_json::json!({
            "batchId": "batch-reinforcement",
            "sourceType": "text",
            "sourceName": "external app",
            "acceptedCandidateIds": ["abandon", "unknownword"],
            "candidates": [
                {
                    "candidateId": "abandon",
                    "word": "abandon",
                    "occurrenceCount": 2,
                    "confidence": 0.8,
                    "evidence": "known imported word"
                },
                {
                    "candidateId": "unknownword",
                    "word": "unknownword",
                    "occurrenceCount": 5,
                    "confidence": 0.7,
                    "isHighFrequency": true,
                    "evidence": "unknown imported word"
                }
            ]
        });
        commit_wrong_word_import_with_connection(&conn, &request).expect("commit import");

        let ids = load_prioritized_wrong_word_entry_ids(&conn, 10).expect("load reinforcement ids");
        assert_eq!(ids, vec![1]);
    }

    #[test]
    fn wrong_word_entries_mark_root_affix_items() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("word-root-affix-wrong-detail-{nonce}"));
        let book_dir = bundle_dir.join("seed-vocab").join("book");
        fs::create_dir_all(&book_dir).expect("create root affix fixture dir");
        fs::write(
            book_dir.join("KaoYan_3.json"),
            serde_json::json!([
                {
                    "headWord": "absorb",
                    "content": {"word": {"wordHead": "absorb", "content": {
                        "trans": [{"tranCn": "吸收"}],
                        "remMethod": {"val": "abs(离开) + orb -> absorb"}
                    }}}
                },
                {
                    "headWord": "abstain",
                    "content": {"word": {"wordHead": "abstain", "content": {
                        "trans": [{"tranCn": "戒除，弃权"}],
                        "remMethod": {"val": "abs(离开) + tain -> abstain"}
                    }}}
                }
            ])
            .to_string(),
        )
        .expect("write root affix fixture");

        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-root-affix-wrong-words', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'root_affix_shared_abs', 'abs', 'root', 1.0)",
            [],
        )
        .expect("insert root entry");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_root_affix', '\"rootAffix\"', 1, '2026-04-30T10:00:00Z', '2026-04-30T10:05:00Z')",
            [],
        )
        .expect("insert study session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_root_affix', 'q_root', 1, '\"rootToGlossInput\"', 'wrong', 'away', '\"incorrect\"', 100, '2026-04-30T10:01:00Z')",
            [],
        )
        .expect("insert root wrong result");

        let mut wrong_words = load_wrong_word_entries(&conn).expect("load wrong words");
        enrich_root_affix_entries_from_assets(&mut wrong_words, &bundle_dir)
            .expect("enrich root affix list");

        assert_eq!(wrong_words.len(), 1);
        assert_eq!(wrong_words[0]["word"], "abs");
        assert_eq!(wrong_words[0]["entryKind"], "rootAffix");
        assert_eq!(wrong_words[0]["partOfSpeech"], "root");
        assert_eq!(wrong_words[0]["meanings"][0], "离开");

        let detail = load_wrong_word_detail_payload_with_bundle(&conn, 1, Some(&bundle_dir))
            .expect("load root affix detail");
        assert_eq!(detail["meanings"][0]["meaningCn"], "离开");
        assert_eq!(detail["examples"][0]["sentenceEn"], "absorb");
        assert_eq!(detail["examples"][0]["sentenceCn"], "吸收");
        assert_eq!(detail["examples"][1]["sentenceEn"], "abstain");

        fs::remove_dir_all(bundle_dir).expect("cleanup root affix fixture");
    }

    #[test]
    fn reports_history_accepts_json_encoded_result_enums() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-report-enums', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'report_entry', 'alpha', 'n.', 1.0)",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_report', '\"mixedTest\"', 1, '2026-04-29T01:00:00Z', '2026-04-29T01:10:00Z')",
            [],
        )
        .expect("insert session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_report', 'q1', 1, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, '2026-04-29T01:01:00Z')",
            [],
        )
        .expect("insert result");

        let history = load_reports_history(&conn).expect("load reports history");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0]["mode"], "mixedTest");
        assert_eq!(history[0]["summary"]["correctCount"], 1);

        let summary = recompute_summary_json(
            "sess_report",
            &[serde_json::json!({
                "questionId": "q1",
                "entrySourceId": "entry_1",
                "questionType": "enToCnChoice",
                "userResponse": "A",
                "normalizedResponse": "A",
                "correctAnswer": "A",
                "outcome": "correct",
                "responseTimeMs": 100,
                "answeredAt": "2026-04-29T01:01:00Z"
            })],
            "2026-04-29T01:10:00Z",
        )
        .expect("recompute summary");
        assert_eq!(summary["correctCount"], 1);
    }

    #[test]
    fn reports_history_uses_answered_dates_even_when_session_is_unfinished() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-report-answer-dates', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'report_entry', 'alpha', 'n.', 1.0)",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_unfinished_report', '\"review\"', 1, '2026-05-01T08:00:00Z', NULL)",
            [],
        )
        .expect("insert unfinished session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_unfinished_report', 'q1', 1, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, '2026-04-30T16:01:00Z'),
                    ('sess_unfinished_report', 'q2', 1, '\"enToCnChoice\"', 'B', 'A', '\"incorrect\"', 100, '2026-04-30T16:02:00Z')",
            [],
        )
        .expect("insert unfinished session results");

        let history = load_reports_history(&conn).expect("load reports history");

        assert_eq!(history.len(), 1);
        assert_eq!(history[0]["date"], "2026-05-01");
        assert_eq!(history[0]["mode"], "review");
        assert_eq!(history[0]["summary"]["totalQuestions"], 2);
        assert_eq!(history[0]["summary"]["correctCount"], 1);
    }

    #[test]
    fn ai_generation_request_accepts_target_words_from_wrong_pool() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-ai-target-words', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'ai_wrong', 'alpha', 'n.', 1.0)",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'n.', 'alpha meaning', 0)",
            [],
        )
        .expect("insert meaning");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_ai_wrong', '\"mixedTest\"', 1, '2026-04-29T01:00:00Z', NULL)",
            [],
        )
        .expect("insert session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_ai_wrong', 'q1', 1, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-04-29T01:01:00Z')",
            [],
        )
        .expect("insert wrong result");

        let direct = resolve_ai_request_wrong_words(&serde_json::json!({
            "wrongWords": [{
                "entryId": 1,
                "word": "alpha",
                "primaryGloss": "alpha meaning",
                "partOfSpeech": "n."
            }]
        }))
        .expect("resolve direct wrong words");
        assert_eq!(direct.len(), 1);
        assert_eq!(direct[0]["word"], "alpha");

        let fallback = load_wrong_word_entries(&conn).expect("load wrong words");
        assert_eq!(fallback.len(), 1);
    }

    #[test]
    fn ai_passage_parser_replaces_negative_entry_id_markers() {
        let parsed = parse_ai_model_output(
            r#"{"title":"Shop note","paragraphs":["The clerk made a [[word:-4]] before lunch."]}"#,
            &[serde_json::json!({
                "entryId": -4,
                "word": "enquiry",
                "primaryGloss": "打听，询问",
                "partOfSpeech": "n."
            })],
        )
        .expect("parse AI passage");

        assert_eq!(parsed["missingWordIds"], serde_json::json!([]));
        assert_eq!(parsed["coveredWordIds"], serde_json::json!([-4]));
        assert_eq!(parsed["blocks"][0]["segments"][1]["type"], "word");
        assert_eq!(parsed["blocks"][0]["segments"][1]["text"], "enquiry");
    }

    #[test]
    fn ai_passage_lookup_finds_existing_passage_for_date() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        set_json_setting(
            &conn,
            "ai_passage_history_json",
            &serde_json::json!([
                {
                    "passageId": "today-pass",
                    "title": "Today",
                    "date": "2026-04-30",
                    "generatedAt": "2026-04-30T08:00:00Z"
                },
                {
                    "passageId": "old-pass",
                    "title": "Old",
                    "date": "2026-04-29",
                    "generatedAt": "2026-04-29T08:00:00Z"
                }
            ]),
        )
        .expect("seed history");

        let found = find_ai_passage_for_date(&conn, "2026-04-30")
            .expect("lookup existing passage")
            .expect("passage for date");
        assert_eq!(found["passageId"], "today-pass");
        assert!(find_ai_passage_for_date(&conn, "2026-05-01")
            .expect("lookup missing date")
            .is_none());
    }

    #[test]
    fn ai_prompt_length_scales_with_wrong_word_count() {
        let words = (1..=15)
            .map(|id| {
                serde_json::json!({
                    "entryId": id,
                    "word": format!("word{id}"),
                    "primaryGloss": format!("meaning {id}"),
                    "partOfSpeech": "n."
                })
            })
            .collect::<Vec<_>>();

        let prompt =
            build_ai_user_prompt(&words, "default", "2026-05-01").expect("build AI prompt");

        assert!(prompt.contains("Target word count: 15"));
        assert!(prompt.contains("Length target: 340-520 Chinese characters"));
        assert!(prompt.contains("Paragraph target: 4 paragraphs"));
    }

    #[test]
    fn today_ai_context_uses_more_than_five_wrong_words() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-ai-context-limit', 'ready')",
            [],
        )
        .expect("insert source version");

        for id in 1..=8 {
            conn.execute(
                "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
                 VALUES (?1, 1, ?2, ?3, 'n.', ?4)",
                rusqlite::params![
                    id,
                    format!("ai_context_wrong_{id}"),
                    format!("wrongword{id}"),
                    1_000.0 - id as f64
                ],
            )
            .expect("insert entry");
            conn.execute(
                "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
                 VALUES (?1, 'n.', ?2, 0)",
                rusqlite::params![id, format!("meaning {id}")],
            )
            .expect("insert meaning");
            conn.execute(
                "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
                 VALUES (?1, '\"mixedTest\"', 1, '2026-05-02T01:00:00Z', NULL)",
                rusqlite::params![format!("sess_ai_context_{id}")],
            )
            .expect("insert session");
            conn.execute(
                "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
                 correct_answer, outcome, response_time_ms, answered_at)
                 VALUES (?1, ?2, ?3, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-02T01:01:00Z')",
                rusqlite::params![
                    format!("sess_ai_context_{id}"),
                    format!("q_ai_context_{id}"),
                    id
                ],
            )
            .expect("insert wrong result");
        }

        let wrong_words = load_wrong_word_inputs(&conn, AI_PASSAGE_MAX_WRONG_WORDS)
            .expect("load AI context wrong words");
        assert_eq!(wrong_words.len(), 8);
    }

    #[test]
    fn ai_wrong_word_inputs_exclude_root_affix_items() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-ai-root-affix-filter', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'normal_word', 'cancel', 'v.', 2.0),
                    (2, 1, 'root_affix_abs', 'abs', 'root', 1.0)",
            [],
        )
        .expect("insert entries");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'v.', 'cancel meaning', 0),
                    (2, 'root', 'away meaning', 0)",
            [],
        )
        .expect("insert meanings");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_word', '\"mixedTest\"', 1, '2026-05-05T01:00:00Z', NULL),
                    ('sess_root', '\"rootAffix\"', 1, '2026-05-05T01:00:00Z', NULL)",
            [],
        )
        .expect("insert sessions");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_word', 'q_word', 1, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-05T01:01:00Z'),
                    ('sess_root', 'q_root', 2, '\"rootToGlossInput\"', 'wrong', 'away', '\"incorrect\"', 100, '2026-05-05T01:02:00Z')",
            [],
        )
        .expect("insert wrong results");

        let wrong_words =
            load_wrong_word_inputs(&conn, AI_PASSAGE_MAX_WRONG_WORDS).expect("load AI inputs");

        assert_eq!(wrong_words.len(), 1);
        assert_eq!(wrong_words[0]["word"], "cancel");
        assert_eq!(wrong_words[0]["primaryGloss"], "cancel meaning");
    }

    #[test]
    fn seed_dedup_merges_same_word_and_pos_within_wordbook() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-seed-dedup', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 2, 1)",
            [],
        )
        .expect("insert wordbook");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'cancel_a', 'cancel', 'v.', 2.0),
                    (2, 1, 'cancel_b', 'cancel', 'vt.', 1.0)",
            [],
        )
        .expect("insert duplicate entries");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'v.', '取消；撤销；删去', 0),
                    (2, 'vt.', '相互抵消', 0)",
            [],
        )
        .expect("insert duplicate meanings");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1), (3, 2, 2)",
            [],
        )
        .expect("insert duplicate links");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_wrong_dup', '\"mixedTest\"', 1, '2026-04-29T01:00:00Z', NULL)",
            [],
        )
        .expect("insert session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_wrong_dup', 'q1', 2, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-04-29T01:01:00Z')",
            [],
        )
        .expect("insert duplicate result");

        repair_seed_vocabulary_dedup(&conn).expect("repair duplicates");

        let linked_count: i64 = conn
            .query_row(
                "SELECT COUNT(*)
                 FROM wordbook_entries we
                 JOIN entries e ON e.id = we.entry_id
                 WHERE we.wordbook_id = 3 AND e.word = 'cancel'",
                [],
                |row| row.get(0),
            )
            .expect("count links");
        assert_eq!(linked_count, 1);

        let meaning_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entry_meanings WHERE entry_id = 1",
                [],
                |row| row.get(0),
            )
            .expect("count meanings");
        assert_eq!(meaning_count, 2);

        let moved_entry_id: i64 = conn
            .query_row(
                "SELECT entry_id FROM study_results WHERE question_id = 'q1'",
                [],
                |row| row.get(0),
            )
            .expect("load moved result");
        assert_eq!(moved_entry_id, 1);

        let wrong_words = load_wrong_word_entries(&conn).expect("load wrong words");
        assert_eq!(wrong_words.len(), 1);
        assert_eq!(wrong_words[0]["entryId"], 1);
    }

    #[test]
    fn today_progress_tracks_partial_and_completed_study_sessions() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        word_app_core::clear_all_active_sessions();
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-today-progress', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 1, 1)",
            [],
        )
        .expect("insert wordbook");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'entry_one', 'alpha', 'n.', 1.0)",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'n.', 'alpha meaning', 0)",
            [],
        )
        .expect("insert meaning");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1)",
            [],
        )
        .expect("insert wordbook entry");
        set_json_setting(
            &conn,
            "today_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select active wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 1,
                "reviewWordsPerDay": 0,
                "mixedTestPerDay": 0,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 0
            }),
        )
        .expect("set today plan");

        let request = word_storage_core::models::StartSessionRequest {
            mode: word_storage_core::models::SessionMode::NewWord,
            wordbook_id: Some(3),
            entry_source_ids: vec!["entry_one".to_string()],
            entry_payloads: vec![word_storage_core::models::StartSessionEntryPayload {
                source_id: "entry_one".to_string(),
                word: "alpha".to_string(),
                part_of_speech: Some("n.".to_string()),
                frequency: 1.0,
                phonetic_us: None,
                phonetic_uk: None,
                meaning_details: vec![word_storage_core::models::StartSessionMeaningPayload {
                    pos: "n.".to_string(),
                    meaning_cn: "alpha meaning".to_string(),
                    meaning_en: None,
                }],
                meanings: vec!["alpha meaning".to_string()],
                example_sentence: Some("Alpha starts.".to_string()),
                example_translation: Some("alpha meaning".to_string()),
            }],
            distractor_payloads: Vec::new(),
        };
        let start = word_app_core::start_study_session(&conn, request).expect("start session");
        let session_id = start.session.session_id.clone();
        let mut question = start.current_question;
        for _ in 0..2 {
            let response = word_app_core::submit_study_answer(
                &conn,
                word_storage_core::models::SubmitAnswerRequest {
                    question_id: question.question_id,
                    response: "A".to_string(),
                    response_time_ms: 100,
                },
            )
            .expect("submit partial answer");
            question = response
                .current_question
                .expect("session should still have question");
        }

        let partial = build_authoritative_today_home_state(&conn).expect("build partial today");
        let partial_snapshot = partial.today_snapshot.expect("partial snapshot");
        assert_eq!(partial_snapshot.new_words_completed, 2);

        loop {
            let response = word_app_core::submit_study_answer(
                &conn,
                word_storage_core::models::SubmitAnswerRequest {
                    question_id: question.question_id,
                    response: "A".to_string(),
                    response_time_ms: 100,
                },
            )
            .expect("submit remaining answer");
            if response.is_complete {
                break;
            }
            question = response
                .current_question
                .expect("incomplete session should have next question");
        }
        word_app_core::complete_study_session(&conn, &session_id).expect("complete session");

        let completed = build_authoritative_today_home_state(&conn).expect("build completed today");
        let completed_snapshot = completed.today_snapshot.expect("completed snapshot");
        assert_eq!(completed_snapshot.new_words_completed, 4);
    }

    #[test]
    fn root_affix_kaoyan_selection_uses_shared_assets_not_medical_fallback() {
        let bundle_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/mobile/android/app/src/main/assets");

        let payloads = super::load_root_affix_payloads_for_active_wordbooks(&bundle_dir, &[3], 5)
            .expect("load kaoyan root/affix payloads");

        assert!(!payloads.is_empty());
        assert!(
            payloads
                .iter()
                .all(|payload| !payload.source_id.starts_with("root_affix_medical_")),
            "KaoYan root/affix mode should not use medical fallback cards"
        );
    }

    #[test]
    fn root_affix_hydration_uses_daily_stable_order_instead_of_id_prefix() {
        let bundle_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/mobile/android/app/src/main/assets");

        let day_one = load_root_affix_payloads_for_active_wordbooks_on_date(
            &bundle_dir,
            &[3],
            4,
            "2026-04-29",
        )
        .expect("load day one root affix payloads");
        let day_one_again = load_root_affix_payloads_for_active_wordbooks_on_date(
            &bundle_dir,
            &[3],
            4,
            "2026-04-29",
        )
        .expect("load day one root affix payloads again");
        let day_two = load_root_affix_payloads_for_active_wordbooks_on_date(
            &bundle_dir,
            &[3],
            4,
            "2026-04-30",
        )
        .expect("load day two root affix payloads");
        let day_three = load_root_affix_payloads_for_active_wordbooks_on_date(
            &bundle_dir,
            &[3],
            4,
            "2026-05-01",
        )
        .expect("load day three root affix payloads");

        let ids = |items: Vec<word_storage_core::models::StartSessionEntryPayload>| {
            items
                .into_iter()
                .map(|payload| payload.source_id)
                .collect::<Vec<_>>()
        };
        let day_one_ids = ids(day_one);
        let day_one_again_ids = ids(day_one_again);
        let day_two_ids = ids(day_two);
        let day_three_ids = ids(day_three);

        assert_eq!(day_one_ids, day_one_again_ids);
        assert!(!day_one_ids.is_empty());
        assert_ne!(day_one_ids, day_two_ids);
        assert_ne!(day_two_ids, day_three_ids);
        assert!(day_one_ids
            .iter()
            .chain(day_two_ids.iter())
            .chain(day_three_ids.iter())
            .all(|id| !id.starts_with("active_session_restore_")));
    }

    #[test]
    fn root_affix_shared_selection_does_not_fallback_to_medical_only_assets() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("word-root-affix-test-{nonce}"));
        let medical_dir = bundle_dir.join("seed-medical");
        fs::create_dir_all(&medical_dir).expect("create medical asset dir");
        fs::write(
            medical_dir.join("medical-root-affix.txt"),
            "cardio 心脏\n例：cardiology(心脏病学)\n",
        )
        .expect("write medical asset");

        let payloads = super::load_root_affix_payloads_for_active_wordbooks(&bundle_dir, &[3], 5)
            .expect("load root/affix payloads");

        assert!(
            payloads.is_empty(),
            "KaoYan/shared root-affix selection must not fall back to medical-only assets"
        );

        fs::remove_dir_all(bundle_dir).expect("cleanup temp bundle dir");
    }

    #[test]
    fn restore_placeholder_ids_are_rehydrated_from_real_root_affix_payloads() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("word-root-affix-restore-test-{nonce}"));
        let book_dir = bundle_dir.join("seed-vocab").join("book");
        fs::create_dir_all(&book_dir).expect("create root affix fixture dir");
        fs::write(
            book_dir.join("KaoYan_3.json"),
            r#"[{
              "headWord":"reactivate",
              "content":{"word":{"wordHead":"reactivate","content":{
                "trans":[{"tranCn":"重新激活"}],
                "remMethod":{"val":"re(重新) + active(活跃) -> 重新激活"}
              }}}
            }]"#,
        )
        .expect("write root affix fixture");
        let clean_root_affix_fixture = serde_json::json!([
            {
                "headWord": "reactivate",
                "content": {"word": {"wordHead": "reactivate", "content": {
                    "trans": [{"tranCn": "\u{91cd}\u{65b0}\u{6fc0}\u{6d3b}"}],
                    "remMethod": {"val": "re(\u{518d}) + active(\u{6d3b}\u{52a8}) -> reactivate"}
                }}}
            },
            {
                "headWord": "rebuild",
                "content": {"word": {"wordHead": "rebuild", "content": {
                    "trans": [{"tranCn": "\u{91cd}\u{5efa}"}],
                    "remMethod": {"val": "re(\u{518d}) + build(\u{5efa}\u{9020}) -> rebuild"}
                }}}
            }
        ]);
        fs::write(
            book_dir.join("KaoYan_3.json"),
            clean_root_affix_fixture.to_string(),
        )
        .expect("overwrite root affix fixture with clean utf8 json");

        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        set_json_setting(
            &conn,
            "saved_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select active wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 0,
                "reviewWordsPerDay": 0,
                "mixedTestPerDay": 0,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 1
            }),
        )
        .expect("set today plan");

        let hydrated = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::RootAffix,
                wordbook_id: None,
                entry_source_ids: vec!["active_session_restore_0".to_string()],
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            Some(bundle_dir.clone()),
        )
        .expect("hydrate root affix restore");

        assert_eq!(hydrated.entry_payloads.len(), 1);
        assert!(hydrated
            .entry_payloads
            .iter()
            .all(|payload| !payload.source_id.starts_with("active_session_restore_")));
        assert!(hydrated
            .entry_payloads
            .iter()
            .all(|payload| payload.source_id.starts_with("root_affix_")));

        fs::remove_dir_all(bundle_dir).expect("cleanup temp bundle dir");
    }

    #[test]
    fn root_affix_shared_cards_require_at_least_two_examples() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("word-root-affix-quality-test-{nonce}"));
        let book_dir = bundle_dir.join("seed-vocab").join("book");
        fs::create_dir_all(&book_dir).expect("create root affix fixture dir");
        fs::write(
            book_dir.join("KaoYan_3.json"),
            r#"[{
              "headWord":"guarantee",
              "content":{"word":{"wordHead":"guarantee","content":{
                "trans":[{"tranCn":"保证；保证书；担保物"}],
                "remMethod":{"val":"guar(保证) + antee -> guarantee"}
              }}}
            },{
              "headWord":"reactivate",
              "content":{"word":{"wordHead":"reactivate","content":{
                "trans":[{"tranCn":"重新激活；恢复"}],
                "remMethod":{"val":"re(再) + active(活动) -> reactivate"}
              }}}
            },{
              "headWord":"rebuild",
              "content":{"word":{"wordHead":"rebuild","content":{
                "trans":[{"tranCn":"重建；重新建立"}],
                "remMethod":{"val":"re(再) + build(建造) -> rebuild"}
              }}}
            }]"#,
        )
        .expect("write root affix fixture");

        let payloads = super::load_root_affix_payloads_for_active_wordbooks(&bundle_dir, &[3], 10)
            .expect("load root affix payloads");

        assert!(
            payloads
                .iter()
                .all(|payload| payload.source_id != "root_affix_shared_guar"),
            "single-example roots should be filtered out"
        );
        let re = payloads
            .iter()
            .find(|payload| payload.source_id == "root_affix_shared_re")
            .expect("multi-example re root should remain");
        assert_eq!(re.example_sentence.as_deref(), Some("reactivate, rebuild"));
        assert_eq!(
            re.example_translation.as_deref(),
            Some("重新激活；恢复, 重建；重新建立")
        );

        fs::remove_dir_all(bundle_dir).expect("cleanup temp bundle dir");
    }

    #[test]
    fn restore_placeholder_ids_are_rehydrated_from_real_new_word_payloads() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-new-word-restore', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 1, 1)",
            [],
        )
        .expect("insert wordbook");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'real_entry', 'alpha', 'n.', 1.0)",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'n.', 'alpha meaning', 0)",
            [],
        )
        .expect("insert meaning");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1)",
            [],
        )
        .expect("insert wordbook entry");
        set_json_setting(
            &conn,
            "saved_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select active wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 1,
                "reviewWordsPerDay": 0,
                "mixedTestPerDay": 0,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 0
            }),
        )
        .expect("set today plan");

        let hydrated = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::NewWord,
                wordbook_id: None,
                entry_source_ids: vec!["active_session_restore_0".to_string()],
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
            },
            None,
        )
        .expect("hydrate new word restore");

        assert_eq!(hydrated.entry_source_ids, vec!["real_entry"]);
        assert_eq!(hydrated.entry_payloads[0].word, "alpha");
    }
}
