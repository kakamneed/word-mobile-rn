//! Native bridge exports for React Native.
//!
//! This module provides the FFI boundary between React Native (JavaScript)
//! and the shared Rust core. Functions here are exported to the native modules
//! on iOS and Android.

use std::env;
use std::sync::{Mutex, MutexGuard};

use once_cell::sync::Lazy;

use word_app_core::{
    bootstrap_with_connection as core_bootstrap_with_connection,
    build_today_home_state as core_build_today_home_state,
    cancel_study_session as core_cancel_study_session,
    complete_study_session as core_complete_study_session,
    get_today_home_state as core_get_today_home_state,
    start_study_session as core_start_study_session,
    submit_study_answer as core_submit_study_answer,
};
use word_storage_core::{models::TodayHomeStateSeed, persistence};

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
    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;

    let state = core_get_today_home_state(&conn)
        .map_err(|e| format!("Failed to get today state: {}", e))?;

    serde_json::to_string(&state).map_err(|e| format!("JSON serialization failed: {}", e))
}

pub fn build_today_home_state(request_json: String) -> Result<String, String> {
    let seed: TodayHomeStateSeed =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let state = core_build_today_home_state(seed);
    serde_json::to_string(&state).map_err(|e| format!("JSON serialization failed: {}", e))
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
        Ok(plan.to_string())
    })
}

pub fn apply_saved_plan_to_today() -> Result<String, String> {
    with_runtime_conn(|conn| {
        ensure_planning_state(conn)?;
        let saved_plan = get_json_setting(conn, "saved_plan_json", &default_plan_json())?;
        let saved_wordbooks = get_json_setting(
            conn,
            "saved_wordbooks_json",
            &default_wordbook_selection_json(),
        )?;
        set_json_setting(conn, "today_plan_json", &saved_plan)?;
        set_json_setting(conn, "today_wordbooks_json", &saved_wordbooks)?;
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
    let wrong_words = request
        .get("wrongWords")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
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
    let client = reqwest::blocking::Client::new();
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
    let client = reqwest::blocking::Client::new();
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
    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;
    let db_path = runtime.paths().database_path();
    let conn = persistence::initialize_database(&db_path)
        .map_err(|e| format!("Failed to initialize database: {}", e))?;
    f(&conn)
}

fn ensure_planning_state(conn: &word_storage_core::Connection) -> Result<(), String> {
    ensure_setting(conn, "saved_plan_json", &default_plan_json())?;
    ensure_setting(conn, "today_plan_json", &default_plan_json())?;
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
    left.get("intervalDays").and_then(|value| value.as_i64()).unwrap_or(7)
        == right
            .get("intervalDays")
            .and_then(|value| value.as_i64())
            .unwrap_or(7)
        && left.get("increment").and_then(|value| value.as_i64()).unwrap_or(5)
            == right
                .get("increment")
                .and_then(|value| value.as_i64())
                .unwrap_or(5)
}

fn growth_rules_by_mode_from_value(value: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
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
            existing.get(mode).cloned().unwrap_or_else(|| shared.clone()),
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
        let previous_rule = previous_rules.get(mode).cloned().unwrap_or_else(|| previous_shared.clone());
        let next_rule = next_rules.get(mode).cloned().unwrap_or_else(|| next_shared.clone());
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
        let value =
            serde_json::to_value(&request).map_err(|e| format!("JSON serialization failed: {e}"))?;
        set_json_setting(conn, "ai_provider_config_json", &value)?;
        let summary = summarize_ai_provider_config(&request);
        serde_json::to_string(&summary).map_err(|e| format!("JSON serialization failed: {}", e))
    })
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

    let response = core_start_study_session(&conn, request)
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
    use super::{call_backup_ai, call_primary_ai};
    use std::env;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

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
}
