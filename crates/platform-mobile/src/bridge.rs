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
        let history = load_reports_history(conn)?;
        let today_date = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let learned_count = load_distinct_learned_count(conn)?;
        let payload = reports_domain::build_reports_overview(&today_date, &history, learned_count);
        serde_json::to_string(&payload).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn get_wrong_words(filter: String) -> Result<String, String> {
    with_runtime_conn(|conn| {
        let mut entries = load_wrong_word_entries(conn)?;
        let payload = wrong_words_domain::build_wrong_words(&mut entries, &filter);
        serde_json::to_string(&payload).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn get_wrong_word_detail(entry_id: i64) -> Result<String, String> {
    with_runtime_conn(|conn| {
        let payload = load_wrong_word_detail_payload(conn, entry_id)?;
        serde_json::to_string(&payload).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn get_today_ai_passage_context() -> Result<String, String> {
    with_runtime(|runtime, conn| {
        ensure_seed_vocabulary_imported(conn, &runtime.paths().bundled_resource_path(""))?;
        let today_state = build_authoritative_today_home_state(conn)?;
        let wrong_words = load_wrong_word_inputs(conn, 5)?;

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
        serde_json::to_string(&payload).map_err(|e| format!("JSON serialization failed: {}", e))
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
        "saved_wordbooks_json",
        &default_wordbook_selection_json(),
    )?;
    set_json_setting(conn, "today_wordbooks_json", &wordbook_value)?;

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
        let payload = load_resume_session_hint(conn)?;
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
    use rusqlite::params;
    use std::collections::BTreeMap;

    let mut sessions_stmt = conn
        .prepare(
            "SELECT session_id, mode, completed_at
             FROM study_sessions
             WHERE completed_at IS NOT NULL
             ORDER BY completed_at ASC",
        )
        .map_err(|e| format!("Failed to prepare reports session query: {e}"))?;

    let sessions = sessions_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("Failed to query reports sessions: {e}"))?;

    let mut history = Vec::new();
    for session in sessions {
        let (session_id, mode, completed_at) =
            session.map_err(|e| format!("Failed to decode reports session row: {e}"))?;
        let mode = normalize_persisted_enum_text(&mode);

        let mut results_stmt = conn
            .prepare(
                "SELECT question_id, entry_id, question_type, user_response, normalized_response,
                        correct_answer, outcome, response_time_ms, answered_at
                 FROM study_results
                 WHERE session_id = ?1
                 ORDER BY id ASC",
            )
            .map_err(|e| format!("Failed to prepare reports result query: {e}"))?;

        let results = results_stmt
            .query_map(params![session_id], |row| {
                let question_type: String = row.get(2)?;
                let outcome: String = row.get(6)?;
                let entry_id: i64 = row.get(1)?;
                Ok(serde_json::json!({
                    "questionId": row.get::<_, String>(0)?,
                    "entrySourceId": format!("entry_{entry_id}"),
                    "questionType": normalize_persisted_enum_text(&question_type),
                    "userResponse": row.get::<_, String>(3)?,
                    "normalizedResponse": row.get::<_, Option<String>>(4)?,
                    "correctAnswer": row.get::<_, String>(5)?,
                    "outcome": normalize_persisted_enum_text(&outcome),
                    "responseTimeMs": row.get::<_, i64>(7)?,
                    "answeredAt": row.get::<_, String>(8)?,
                }))
            })
            .map_err(|e| format!("Failed to query reports results: {e}"))?;

        let mut session_results = Vec::new();
        for result in results {
            session_results
                .push(result.map_err(|e| format!("Failed to decode reports result row: {e}"))?);
        }

        let summary_value = recompute_summary_json(&session_id, &session_results, &completed_at)?;
        let date = completed_at.get(0..10).unwrap_or(&completed_at).to_string();

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

fn normalize_persisted_enum_text(value: &str) -> String {
    serde_json::from_str::<String>(value).unwrap_or_else(|_| value.to_string())
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
                e.word,
                e.phonetic_us,
                e.phonetic_uk,
                SUM(CASE WHEN sr.outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"') THEN 1 ELSE 0 END) as error_count,
                MAX(CASE WHEN sr.outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"') THEN sr.answered_at ELSE NULL END) as last_wrong_at
             FROM study_results sr
             JOIN entries e ON e.id = sr.entry_id
             WHERE sr.outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"')
             GROUP BY e.id, e.word, e.phonetic_us, e.phonetic_uk
             ORDER BY last_wrong_at DESC",
        )
        .map_err(|e| format!("Failed to prepare wrong-word list query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let entry_id: i64 = row.get(0)?;
            Ok((
                entry_id,
                serde_json::json!({
                    "entryId": entry_id,
                    "word": row.get::<_, String>(1)?,
                    "phoneticUs": row.get::<_, Option<String>>(2)?,
                    "phoneticUk": row.get::<_, Option<String>>(3)?,
                    "errorCount": row.get::<_, i64>(4)?,
                    "lastWrongAt": row.get::<_, Option<String>>(5)?,
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
        result.push(value);
    }
    Ok(result)
}

fn load_wrong_word_detail_payload(
    conn: &rusqlite::Connection,
    entry_id: i64,
) -> Result<serde_json::Value, String> {
    let (word, lemma, phonetic_us, phonetic_uk, part_of_speech) = conn
        .query_row(
            "SELECT word, lemma, phonetic_us, phonetic_uk, part_of_speech
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
                ))
            },
        )
        .map_err(|e| format!("Failed to load wrong-word entry detail: {e}"))?;

    let meanings = load_entry_meaning_details(conn, entry_id)?;
    let examples = load_entry_examples(conn, entry_id)?;

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

    Ok(serde_json::json!({
        "entryId": entry_id,
        "word": word,
        "lemma": lemma,
        "phoneticUs": phonetic_us,
        "phoneticUk": phonetic_uk,
        "partOfSpeech": part_of_speech.unwrap_or_default(),
        "meanings": meanings,
        "examples": examples,
        "errorHistory": error_history,
        "riskBreakdown": risk_breakdown,
        "relatedWords": Vec::<String>::new()
    }))
}

fn load_wrong_word_inputs(
    conn: &rusqlite::Connection,
    limit: usize,
) -> Result<Vec<serde_json::Value>, String> {
    let mut entries = load_wrong_word_entries(conn)?;
    let ordered = wrong_words_domain::build_wrong_words(&mut entries, "all");
    let mut result = Vec::new();

    for entry in ordered.into_iter().take(limit) {
        let entry_id = entry
            .get("entryId")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        let part_of_speech = conn
            .query_row(
                "SELECT part_of_speech FROM entries WHERE id = ?1",
                [entry_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .unwrap_or(None);
        let primary_gloss = load_entry_meaning_strings(conn, entry_id)?
            .into_iter()
            .find_map(|value| value.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

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
        .map(|values| values.into_iter().map(serde_json::Value::String).collect())
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
            Ok(serde_json::json!({
                "pos": row.get::<_, String>(0)?,
                "meaningCn": row.get::<_, String>(1)?,
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
        let outbox_payload_json = outbox_payload.to_string();
        if let Err(error) = persistence::sync_repo::enqueue_latest_outbox_item(
            conn,
            "plan_config",
            &outbox_payload_json,
            "plan_config:saved_plan_json",
        ) {
            let _ = persistence::sync_repo::record_dead_letter(
                conn,
                "plan_config",
                &outbox_payload_json,
                "plan_config:saved_plan_json",
                "enqueue_failed",
                &error.to_string(),
            );
        }
        Ok(plan.to_string())
    })
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
        persist_today_target_seed(conn, &today_date, &saved_plan)?;
        Ok(saved_plan.to_string())
    })
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
        let mut selection = get_json_setting(
            conn,
            "saved_wordbooks_json",
            &default_wordbook_selection_json(),
        )?;
        if let Some(object) = selection.as_object_mut() {
            object.insert(wordbook_id.to_string(), serde_json::Value::Bool(is_active));
        }
        set_json_setting(conn, "saved_wordbooks_json", &selection)?;
        set_json_setting(conn, "today_wordbooks_json", &selection)?;
        Ok(())
    })
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
            .ok_or("passageId is required")?;
        history_array.retain(|item| {
            item.get("passageId")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                != passage_id
        });
        history_array.insert(0, passage);
        set_json_setting(conn, "ai_passage_history_json", &history)?;
        Ok(())
    })
}

pub fn get_ai_passage_history() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let history = get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
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
        let history = get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
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

pub fn generate_ai_passage(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let date = request
        .get("date")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let level = request
        .get("level")
        .and_then(|value| value.as_str())
        .unwrap_or("intermediate")
        .to_string();
    let wrong_words = resolve_ai_request_wrong_words(&request)?;
    if wrong_words.is_empty() {
        return Err("No wrong words available for passage generation".to_string());
    }

    let style = "default";
    let system_message =
        format!("{PROMPT_TEMPLATE}\n\n## Active Style\n\n{DEFAULT_STYLE_TEMPLATE}");
    let user_message = build_ai_user_prompt(&wrong_words, style, &date)?;

    let raw_content = call_primary_ai(&system_message, &user_message)
        .or_else(|primary_error| call_backup_ai(&system_message, &user_message, &primary_error))?;
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
        .cloned()
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
        candidates.truncate(6);
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
const ANTHROPIC_MESSAGES_PATH: &str = "/v1/messages";
const OPENAI_RESPONSES_PATH: &str = "/v1/responses";
const PROMPT_TEMPLATE: &str = "You are a Chinese language learning assistant. Your task is to write a short, readable Chinese passage around specific English vocabulary words provided by the user.\n\nImportant: the backend will insert the actual English words and Chinese glosses. You should only decide where each word belongs inside the Chinese passage.\n\nYou will receive:\n- word_list: a JSON array of objects with word, primary_gloss, part_of_speech, entry_id\n- style: the desired writing style\n- length_target: approximate character count for the Chinese body text\n\nYou MUST return exactly one JSON object with this structure:\n{\"title\":\"Optional contextual title\",\"paragraphs\":[\"Chinese paragraph with markers such as [[word:101]] inside the text.\"]}\n\nRules:\n1. Return exactly one JSON object and nothing else.\n2. Use [[word:ENTRY_ID]] exactly once per target word.\n3. Do not output the English target words or glosses directly.\n4. Write natural Chinese paragraphs, not a word list.\n5. A loose coherent scene is enough; do not force dramatic plot twists.";
const DEFAULT_STYLE_TEMPLATE: &str = "Writing tone: Informative and accessible, similar to a short textbook note\nSentence length: Short to medium\nVocabulary level: Common Chinese vocabulary\nTopic connection: A loose coherent scene is enough\nParagraph structure: 2-3 paragraphs\nCharacter count target: 120-240 Chinese characters";

#[derive(Clone)]
struct AnthropicAiConfig {
    url: String,
    model: String,
    key: String,
}

#[derive(Clone)]
struct OpenAiResponsesConfig {
    url: String,
    model: String,
    key: String,
}

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

    let length_target = match wrong_words.len() {
        0..=3 => "90-140 characters",
        4..=6 => "120-180 characters",
        7..=10 => "150-240 characters",
        _ => "180-280 characters",
    };

    Ok(format!(
        "Generate a Chinese passage plan for the following English vocabulary words.\n\n## Word List\n```json\n{word_list_json}\n```\n\n## Parameters\n- Style: {style}\n- Prompt version: 2.1\n- Length target: {length_target}\n- Date: {date}\n\nRemember: write natural Chinese paragraphs and place each word exactly once using [[word:entry_id]] markers. Do not output the English words or glosses directly; the backend will inject them. Return only the JSON structure specified in the prompt template."
    ))
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

fn normalize_provider_url(base_url: &str, path_suffix: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with(path_suffix) {
        trimmed.to_string()
    } else {
        format!("{trimmed}{path_suffix}")
    }
}

fn base_url_without_suffix(url: &str, path_suffix: &str) -> String {
    url.trim_end_matches('/')
        .strip_suffix(path_suffix)
        .unwrap_or(url)
        .trim_end_matches('/')
        .to_string()
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

fn primary_ai_config() -> AnthropicAiConfig {
    let config = resolve_ai_provider_config();
    AnthropicAiConfig {
        url: normalize_provider_url(&config.primary.base_url, ANTHROPIC_MESSAGES_PATH),
        model: config.primary.model,
        key: config.primary.auth_token,
    }
}

fn call_primary_ai(system_message: &str, user_message: &str) -> Result<String, String> {
    let config = primary_ai_config();
    call_anthropic_ai(
        &config,
        system_message,
        user_message,
        "Primary AI request failed",
    )
}

fn call_anthropic_ai(
    config: &AnthropicAiConfig,
    system_message: &str,
    user_message: &str,
    error_prefix: &str,
) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| format!("{error_prefix}: failed to build HTTP client: {e}"))?;
    let response = client
        .post(&config.url)
        .header("x-api-key", &config.key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": &config.model,
            "max_tokens": 2048,
            "system": system_message,
            "messages": [{ "role": "user", "content": user_message }],
        }))
        .send()
        .map_err(|e| format!("{error_prefix}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("{error_prefix}: {}", response.status()));
    }
    let json: serde_json::Value = response
        .json()
        .map_err(|e| format!("{error_prefix}: response decode failed: {e}"))?;
    json.get("content")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find_map(|item| {
                if item.get("type").and_then(|value| value.as_str()) == Some("text") {
                    item.get("text").and_then(|value| value.as_str())
                } else {
                    None
                }
            })
        })
        .map(|text| text.to_string())
        .ok_or_else(|| format!("{error_prefix}: response missing text content"))
}

fn call_backup_ai(
    system_message: &str,
    user_message: &str,
    primary_error: &str,
) -> Result<String, String> {
    let resolved = resolve_ai_provider_config();
    let config = OpenAiResponsesConfig {
        url: normalize_provider_url(&resolved.backup.base_url, OPENAI_RESPONSES_PATH),
        model: resolved.backup.model,
        key: resolved.backup.auth_token,
    };
    call_openai_backup_ai(&config, system_message, user_message, primary_error)
}

fn call_openai_backup_ai(
    config: &OpenAiResponsesConfig,
    system_message: &str,
    user_message: &str,
    upstream_error: &str,
) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .map_err(|e| format!("Backup AI client failed after {upstream_error}: {e}"))?;
    let response = client
        .post(&config.url)
        .header("Authorization", format!("Bearer {}", config.key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": &config.model,
            "instructions": system_message,
            "reasoning": { "effort": "low" },
            "input": user_message,
            "store": false,
            "text": {
                "format": { "type": "json_object" },
                "verbosity": "medium",
            }
        }))
        .send()
        .map_err(|e| format!("Backup AI request failed after {upstream_error}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Backup AI request failed after {upstream_error}: {}",
            response.status()
        ));
    }
    let json: serde_json::Value = response
        .json()
        .map_err(|e| format!("Backup AI response decode failed after {upstream_error}: {e}"))?;
    json.get("output")
        .and_then(|value| value.as_array())
        .and_then(|items| {
            items.iter().find_map(|item| {
                item.get("content")
                    .and_then(|value| value.as_array())
                    .and_then(|content| {
                        content.iter().find_map(|part| {
                            if part.get("type").and_then(|value| value.as_str())
                                == Some("output_text")
                            {
                                part.get("text").and_then(|value| value.as_str())
                            } else {
                                None
                            }
                        })
                    })
            })
        })
        .map(|text| text.to_string())
        .ok_or_else(|| format!("Backup AI response missing output_text after {upstream_error}"))
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
    let marker = regex::Regex::new(r"\[\[word:(\d+)\]\]").map_err(|e| e.to_string())?;
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

fn strip_code_fences(content: &str) -> String {
    let trimmed = content.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string()
}

fn extract_json_payload(content: &str) -> String {
    let start = content.find('{').unwrap_or(0);
    let slice = &content[start..];
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in slice.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return slice[..=index].to_string();
                }
            }
            _ => {}
        }
    }
    slice.to_string()
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
            .unwrap_or("")
            .trim();
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
                meaning_cn,
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

fn repair_seed_vocabulary_dedup(conn: &word_storage_core::Connection) -> Result<(), String> {
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
    let new_words = plan_value
        .get("newWordsPerDay")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0) as u32
        * 4;
    let review_words = plan_value
        .get("reviewWordsPerDay")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0) as u32
        * 4;
    let mixed_test = plan_value
        .get("mixedTestPerDay")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0) as u32;
    let wrong_word_test = plan_value
        .get("wrongWordTestPerDay")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0) as u32;
    let root_affix = plan_value
        .get("rootAffixPerDay")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0) as u32;

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
    let new_word_units = plan_unit_count(plan_value, "newWordsPerDay");
    let review_units = plan_unit_count(plan_value, "reviewWordsPerDay");
    let mixed_units = plan_unit_count(plan_value, "mixedTestPerDay");
    let wrong_word_units = plan_unit_count(plan_value, "wrongWordTestPerDay");
    let root_affix_units = plan_unit_count(plan_value, "rootAffixPerDay");

    let available_new_words = load_ranked_entry_ids_for_wordbooks_with_global_fallback(
        conn,
        &active_wordbook_ids,
        new_word_units,
    )?
    .len() as u32;
    let available_review_words =
        load_learned_entry_ids_for_wordbooks(conn, &active_wordbook_ids, review_units)?.len()
            as u32;
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
        let status = persistence::sync_repo::get_sync_status(conn)
            .map_err(|e| format!("Failed to get sync status: {e}"))?;
        serde_json::to_string(&status).map_err(|e| format!("JSON serialization failed: {}", e))
    })
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

    let request = apply_active_session_restore_shape(&conn, request)?;

    let hydrated_request = hydrate_start_session_request(
        &conn,
        request,
        Some(runtime.paths().bundled_resource_path("")),
    )?;

    let response = core_start_study_session(&conn, hydrated_request)
        .map_err(|e| format!("Failed to start session: {}", e))?;

    serde_json::to_string(&response).map_err(|e| format!("JSON serialization failed: {}", e))
}

fn apply_active_session_restore_shape(
    conn: &word_storage_core::Connection,
    mut request: StartSessionRequest,
) -> Result<StartSessionRequest, String> {
    if !request.entry_source_ids.is_empty() || !request.entry_payloads.is_empty() {
        return Ok(request);
    }

    let Some(raw) = persistence::study_repo::load_active_session_snapshot(conn, &request.mode)
        .map_err(|e| format!("Failed to load active session snapshot: {e}"))?
    else {
        return Ok(request);
    };

    let snapshot: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("Invalid active session snapshot: {e}"))?;
    if snapshot
        .get("questionEngineVersion")
        .and_then(|value| value.as_i64())
        != Some(3)
    {
        return Ok(request);
    }

    let question_count = snapshot
        .get("questions")
        .and_then(|value| value.as_array())
        .map(|items| items.len())
        .unwrap_or(0);
    let entry_count = match request.mode {
        SessionMode::NewWord | SessionMode::Review => question_count / 4,
        _ => question_count,
    };
    if entry_count == 0 {
        return Ok(request);
    }

    request.entry_source_ids = (0..entry_count)
        .map(|index| format!("active_session_restore_{index}"))
        .collect();
    Ok(request)
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
                load_distractor_payloads_excluding_sources(conn, &excluded_source_ids, 24)?;
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
            load_learned_entry_ids_for_wordbooks(conn, &active_wordbook_ids, target_count)?
        }
        SessionMode::MixedTest => load_random_entry_ids_for_wordbooks_with_global_fallback(
            conn,
            &active_wordbook_ids,
            target_count,
        )?,
        SessionMode::WrongWordReinforcement => {
            load_prioritized_wrong_word_entry_ids(conn, target_count)?
        }
        SessionMode::NewWord => load_ranked_entry_ids_for_wordbooks_with_global_fallback(
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

    let distractor_target = std::cmp::max(target_count.saturating_mul(4), 24);
    let distractor_ids = load_ranked_entry_ids_for_wordbooks_excluding(
        conn,
        &active_wordbook_ids,
        &entry_ids,
        distractor_target,
    )?;
    let distractor_payloads = load_entry_payloads(conn, &distractor_ids)?;

    request.wordbook_id = request
        .wordbook_id
        .or_else(|| active_wordbook_ids.first().copied());
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
    let count = match mode {
        SessionMode::NewWord => plan
            .get("newWordsPerDay")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        SessionMode::Review => plan
            .get("reviewWordsPerDay")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        SessionMode::MixedTest => plan
            .get("mixedTestPerDay")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        SessionMode::WrongWordReinforcement => plan
            .get("wrongWordTestPerDay")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
        SessionMode::RootAffix => plan
            .get("rootAffixPerDay")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
    };
    Ok(count.max(0) as usize)
}

fn selected_wordbook_ids_for_today(
    conn: &word_storage_core::Connection,
) -> Result<Vec<i64>, String> {
    let saved_selection = get_json_setting(
        conn,
        "saved_wordbooks_json",
        &default_wordbook_selection_json(),
    )?;
    let saved_ids = active_wordbook_ids_from_selection(&saved_selection);
    if !saved_ids.is_empty() {
        return Ok(saved_ids);
    }

    let today_selection = get_json_setting(
        conn,
        "today_wordbooks_json",
        &default_wordbook_selection_json(),
    )?;
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
        .collect())
}

#[derive(Clone)]
struct RootAffixCard {
    id: String,
    form: String,
    meaning_cn: String,
    example_words: String,
    example_glosses: String,
    scope: String,
    example_count: usize,
}

fn load_root_affix_payloads_for_active_wordbooks(
    bundle_dir: &Path,
    active_wordbook_ids: &[i64],
    limit: usize,
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
    scoped.sort_by(|left, right| left.id.cmp(&right.id));
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
            example_words: word.to_string(),
            example_glosses: gloss.to_string(),
            scope: "shared".to_string(),
            example_count: 1,
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
                let (words, glosses) = split_medical_examples(payload);
                cards[index].example_words = words;
                cards[index].example_glosses = glosses;
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
            example_words: String::new(),
            example_glosses: String::new(),
            scope: "medical".to_string(),
            example_count: 1,
        });
        last_index = cards.len().checked_sub(1);
    }
    Ok(cards)
}

fn root_affix_card_to_payload(card: RootAffixCard) -> StartSessionEntryPayload {
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
        example_sentence: non_empty_string(limit_root_examples(&card.example_words, 3)),
        example_translation: non_empty_string(limit_root_examples(&card.example_glosses, 3)),
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
        existing.example_words =
            merge_example_text(&existing.example_words, &incoming.example_words);
        existing.example_glosses =
            merge_example_text(&existing.example_glosses, &incoming.example_glosses);
        existing.example_count += incoming.example_count;
    } else {
        by_id.insert(incoming.id.clone(), incoming);
    }
}

fn merge_example_text(current: &str, incoming: &str) -> String {
    let mut merged = BTreeSet::new();
    for value in current.split(',').chain(incoming.split(',')) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            merged.insert(trimmed.to_string());
        }
    }
    merged.into_iter().collect::<Vec<_>>().join(", ")
}

fn limit_root_examples(value: &str, limit: usize) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .take(limit)
        .collect::<Vec<_>>()
        .join(", ")
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
    if card.scope == "shared"
        && normalized.len() > 4
        && (card.form.starts_with('-') || card.form.ends_with('-'))
    {
        return false;
    }
    if !contains_han(&card.meaning_cn) || sanitize_chinese_meaning(&card.meaning_cn).is_empty() {
        return false;
    }
    card.scope == "medical"
        || card.form.starts_with('-')
        || card.form.ends_with('-')
        || card.example_count >= 1
}

fn split_medical_examples(payload: &str) -> (String, String) {
    let mut words = Vec::new();
    let mut glosses = Vec::new();
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
            words.push(word.trim().to_string());
            glosses.push(gloss.trim().to_string());
        }
    }
    (words.join(", "), glosses.join(", "))
}

fn load_learned_entry_ids_for_wordbooks(
    conn: &word_storage_core::Connection,
    wordbook_ids: &[i64],
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mut conditions =
        "sr.outcome IN ('correct', 'fuzzyCorrect', '\"correct\"', '\"fuzzyCorrect\"')".to_string();
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
        "SELECT e.id, MAX(sr.answered_at) AS last_answered_at
         FROM study_results sr
         JOIN entries e ON e.id = sr.entry_id OR e.source_entry_key = CAST(sr.entry_id AS TEXT)
         {wordbook_join}
         WHERE {conditions}
         GROUP BY e.id
         ORDER BY last_answered_at DESC, e.frequency DESC, e.id ASC
         LIMIT {limit}"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare learned entry seed query: {e}"))?;
    let param_refs = wordbook_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();
    let rows = stmt
        .query_map(&param_refs[..], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query learned entry seed: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode learned entry seed row: {e}"))
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
    if ids.is_empty() {
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
    if ids.is_empty() {
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

    let response = core_submit_study_answer(&conn, request)
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

    let response = core_complete_study_session(&conn, &session_id)
        .map_err(|e| format!("Failed to complete session: {}", e))?;

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
        align_today_targets_to_available_pools, build_authoritative_today_home_state,
        call_backup_ai, call_primary_ai, ensure_planning_state, ensure_seed_vocabulary_imported,
        hydrate_start_session_request, load_reports_history, load_today_completion_seed,
        load_wrong_word_entries, normalize_stored_session_mode, recompute_summary_json,
        repair_seed_vocabulary_dedup, resolve_ai_request_wrong_words, set_json_setting,
        today_target_seed_from_plan_value,
    };
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

        let result = call_primary_ai("system", "user")
            .or_else(|primary_error| call_backup_ai("system", "user", &primary_error))
            .expect("backup path should succeed");

        assert_eq!(result, r#"{"title":"fallback","paragraphs":["ok"]}"#);
    }

    #[test]
    fn today_completion_includes_unfinished_active_session_progress() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let today = "2026-04-28";
        let snapshot = serde_json::json!({
            "questionEngineVersion": 3,
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
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_learned', '\"newWord\"', 2, '2026-04-29T01:00:00Z', '2026-04-29T01:10:00Z')",
            [],
        )
        .expect("insert session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_learned', 'q1', 1, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, '2026-04-29T01:01:00Z'),
                    ('sess_learned', 'q2', 3, '\"enToCnChoice\"', 'A', 'A', '\"correct\"', 100, '2026-04-29T01:02:00Z')",
            [],
        )
        .expect("insert results");

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
        assert_eq!(mixed_ids, vec!["learned_current", "unlearned_current"]);
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
}
