//! Native bridge exports for React Native.
//!
//! This module provides the FFI boundary between React Native (JavaScript)
//! and the shared Rust core. Functions here are exported to the native modules
//! on iOS and Android.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use once_cell::sync::Lazy;
use rusqlite::OptionalExtension;

use word_app_core::services::exam_practice_service as exam_practice_domain;
use word_app_core::services::reports_service as reports_domain;
use word_app_core::services::wrong_words_service as wrong_words_domain;
use word_app_core::{
    accept_disputed_meaning as core_accept_disputed_meaning,
    bootstrap_with_connection as core_bootstrap_with_connection,
    build_today_home_state as core_build_today_home_state,
    cancel_study_session as core_cancel_study_session,
    clear_all_active_sessions as core_clear_all_active_sessions,
    complete_study_session as core_complete_study_session,
    mark_study_entry_mastered_with_replacements as core_mark_study_entry_mastered_with_replacements,
    start_study_session as core_start_study_session,
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
static ROOT_AFFIX_CARD_CACHE: Lazy<Mutex<BTreeMap<String, Vec<RootAffixCard>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));
static EXAM_CORPUS_DICTIONARY_CACHE: Lazy<Mutex<BTreeMap<String, CachedExamCorpusDictionary>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));
static EXAM_DERIVATIONAL_FAMILY_CACHE: Lazy<
    Mutex<BTreeMap<String, BTreeMap<String, serde_json::Value>>>,
> = Lazy::new(|| Mutex::new(BTreeMap::new()));
static STUDY_HIGH_FREQUENCY_SYNONYM_CACHE: Lazy<
    Mutex<BTreeMap<(String, i64), Vec<serde_json::Value>>>,
> = Lazy::new(|| Mutex::new(BTreeMap::new()));
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

fn exam_asset_dir(runtime: &MobileRuntime) -> PathBuf {
    let candidates = [
        runtime.paths().bundled_resource_path("exam-papers"),
        runtime
            .paths()
            .bundled_resource_path("flutter_assets/assets/exam-papers"),
        runtime
            .paths()
            .bundled_resource_path("Frameworks/App.framework/flutter_assets/assets/exam-papers"),
    ];
    candidates
        .iter()
        .find(|path| path.join("manifest.json").is_file())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

fn exam_dictionary_asset_path(bundle_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        bundle_dir
            .join("exam-dictionary")
            .join("exam-corpus-dictionary.json"),
        bundle_dir
            .join("flutter_assets/assets/exam-dictionary")
            .join("exam-corpus-dictionary.json"),
        bundle_dir
            .join("Frameworks/App.framework/flutter_assets/assets/exam-dictionary")
            .join("exam-corpus-dictionary.json"),
    ];
    candidates.into_iter().find(|path| path.is_file())
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
        ensure_seed_vocabulary_available_for_today(
            conn,
            &runtime.paths().bundled_resource_path(""),
        )?;
        let state = build_authoritative_today_home_state(conn)?;
        serde_json::to_string(&state).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

pub fn get_exam_catalog() -> Result<String, String> {
    with_runtime(|runtime, conn| {
        let asset_dir = exam_asset_dir(runtime);
        let mut catalog = exam_practice_domain::load_exam_catalog(&asset_dir)?;
        let user_papers = load_user_exam_papers(conn)?;
        exam_practice_domain::merge_user_papers(&mut catalog, &user_papers);
        serde_json::to_string(&catalog)
            .map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

pub fn get_exam_paper(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let exam = request
        .get("exam")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Missing exam".to_string())?;
    let paper_id = request
        .get("paperId")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Missing paperId".to_string())?;
    with_runtime(|runtime, conn| {
        if let Some(paper) = load_user_exam_paper(conn, exam, paper_id)? {
            return serde_json::to_string(&paper)
                .map_err(|error| format!("JSON serialization failed: {error}"));
        }
        let asset_dir = exam_asset_dir(runtime);
        let paper = exam_practice_domain::load_exam_paper(&asset_dir, exam, paper_id)?
            .ok_or_else(|| format!("Exam paper not found: {exam}/{paper_id}"))?;
        serde_json::to_string(&paper).map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

fn load_user_exam_papers(
    conn: &rusqlite::Connection,
) -> Result<Vec<exam_practice_domain::ExamPaper>, String> {
    let mut statement = conn
        .prepare("SELECT paper_json FROM user_exam_papers ORDER BY updated_at DESC")
        .map_err(|error| format!("Failed to prepare user paper query: {error}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Failed to load user papers: {error}"))?;
    let mut papers = Vec::new();
    for row in rows {
        let raw = row.map_err(|error| format!("Failed to read user paper: {error}"))?;
        let value = serde_json::from_str(&raw)
            .map_err(|error| format!("Stored user paper is invalid: {error}"))?;
        papers.push(exam_practice_domain::normalize_user_paper(value)?);
    }
    Ok(papers)
}

fn load_user_exam_paper(
    conn: &rusqlite::Connection,
    exam: &str,
    paper_id: &str,
) -> Result<Option<exam_practice_domain::ExamPaper>, String> {
    let raw = conn
        .query_row(
            "SELECT paper_json FROM user_exam_papers WHERE exam = ?1 AND paper_id = ?2",
            rusqlite::params![exam, paper_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("Failed to load user paper: {error}"))?;
    raw.map(|raw| {
        let value = serde_json::from_str(&raw)
            .map_err(|error| format!("Stored user paper is invalid: {error}"))?;
        exam_practice_domain::normalize_user_paper(value)
    })
    .transpose()
}

pub fn analyze_exam_paper_import(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let source_type = required_string_field(&request, "sourceType")?;
    let source_name = required_string_field(&request, "sourceName")?;
    let system_message = "You normalize English exam papers. Return JSON only with keys paper and warnings. paper must use schemaVersion 1 and contain id, exam, title, year, optional month/set, source, and sections. Each section has id, type, title, instructions, passage, and questions. Each question has id, number, kind, stem, choices [{label,text}], answer or null, explanation, source, and answerSource. Never invent an answer; use null when absent.";
    let raw = if source_type == "image" {
        let bytes = required_string_field(&request, "bytesBase64")?;
        let mime = request
            .get("mimeType")
            .and_then(|value| value.as_str())
            .unwrap_or("image/jpeg");
        ai_agent().run_image_json(
            system_message,
            &format!("Normalize this exam-paper image. Source name: {source_name}"),
            ImageInput {
                mime_type: mime,
                bytes_base64: &bytes,
            },
        )?
    } else {
        let text = required_string_field(&request, "textContent")?;
        let truncated = text.chars().take(24000).collect::<String>();
        ai_agent().run_text_json(
            system_message,
            &format!("Normalize this UTF-8 exam paper from {source_name}:\n\n{truncated}"),
        )?
    };
    normalize_exam_import_ai_output(&raw, &source_type, &source_name)
}

fn normalize_exam_import_ai_output(
    content: &str,
    source_type: &str,
    source_name: &str,
) -> Result<String, String> {
    let cleaned = extract_json_payload(&strip_code_fences(content));
    let mut value: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|error| format!("AI exam import JSON decode failed: {error}"))?;
    let paper_value = value
        .get_mut("paper")
        .ok_or_else(|| "AI exam import did not return a paper draft".to_string())?;
    let mut paper = exam_practice_domain::normalize_user_paper(paper_value.take())?;
    paper.source = serde_json::json!({
        "sourceType": source_type,
        "sourceName": source_name,
        "origin": "user_import",
        "rawMediaRetained": false
    });
    let warnings = value
        .get("warnings")
        .and_then(|item| item.as_array())
        .cloned()
        .unwrap_or_default();
    serde_json::to_string(&serde_json::json!({
        "paper": paper,
        "warnings": warnings,
        "provenance": paper.source,
        "rawMediaRetained": false
    }))
    .map_err(|error| format!("JSON serialization failed: {error}"))
}

pub fn save_user_exam_paper(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let paper_value = request
        .get("paper")
        .cloned()
        .ok_or_else(|| "Missing paper".to_string())?;
    let paper = exam_practice_domain::normalize_user_paper(paper_value)?;
    let source_type = paper
        .source
        .get("sourceType")
        .and_then(|value| value.as_str())
        .unwrap_or("manual");
    let source_name = paper
        .source
        .get("sourceName")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let warnings = request
        .get("warnings")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let paper_json = serde_json::to_string(&paper)
        .map_err(|error| format!("Failed to serialize user paper: {error}"))?;
    with_runtime_conn(|conn| {
        conn.execute(
            "INSERT INTO user_exam_papers (paper_id, exam, title, paper_json, source_type, source_name, warnings_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(paper_id) DO UPDATE SET
                exam = excluded.exam,
                title = excluded.title,
                paper_json = excluded.paper_json,
                source_type = excluded.source_type,
                source_name = excluded.source_name,
                warnings_json = excluded.warnings_json,
                updated_at = datetime('now')",
            rusqlite::params![
                paper.id,
                paper.exam,
                paper.title,
                paper_json,
                source_type,
                source_name,
                warnings.to_string()
            ],
        )
        .map_err(|error| format!("Failed to save user paper: {error}"))?;
        serde_json::to_string(&serde_json::json!({"saved": true, "paper": paper}))
            .map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

pub fn get_exam_vocabulary_priority() -> Result<String, String> {
    with_runtime(|runtime, conn| {
        let asset_dir = exam_asset_dir(runtime);
        let mut papers = exam_practice_domain::load_all_exam_papers(&asset_dir)?;
        papers.extend(load_user_exam_papers(conn)?);
        let frequencies = exam_practice_domain::rank_exam_vocabulary(&papers, 200);
        let source_catalog = papers
            .iter()
            .flat_map(|paper| {
                paper.sections.iter().map(move |section| {
                    (
                        (paper.id.clone(), section.id.clone()),
                        (paper.title.clone(), section.title.clone()),
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();

        let mut source_statement = conn
            .prepare(
                "SELECT o.normalized_form, a.article_id, a.title, a.metadata_json,
                        SUM(CASE WHEN o.mark_level = 'fuzzy' OR
                            (o.mark_level = 'none' AND o.user_mark = 'ignored') THEN 1 ELSE 0 END),
                        SUM(CASE WHEN o.mark_level = 'familiar' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN o.mark_level = 'unknown' OR
                            (o.mark_level = 'none' AND o.user_mark = 'unknown') THEN 1 ELSE 0 END),
                        SUM(CASE WHEN o.user_mark = 'wrong' THEN 1 ELSE 0 END)
                 FROM exercise_vocab_occurrences o
                 JOIN exercise_articles a ON a.id = o.article_id
                 WHERE o.user_mark IN ('ignored', 'unknown', 'wrong')
                    OR o.mark_level IN ('fuzzy', 'familiar', 'unknown')
                 GROUP BY o.normalized_form, a.article_id, a.title, a.metadata_json
                 ORDER BY MAX(o.updated_at) DESC",
            )
            .map_err(|error| format!("Failed to prepare vocabulary source query: {error}"))?;
        let source_rows = source_statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                ))
            })
            .map_err(|error| format!("Failed to load vocabulary sources: {error}"))?;
        let mut source_evidence = BTreeMap::<String, Vec<serde_json::Value>>::new();
        for row in source_rows {
            let (word, article_id, article_title, metadata_json, fuzzy, familiar, unknown, wrong) =
                row.map_err(|error| format!("Failed to read vocabulary source: {error}"))?;
            let metadata: serde_json::Value =
                serde_json::from_str(&metadata_json).unwrap_or_else(|_| serde_json::json!({}));
            let fallback_ids = article_id.split_once(':');
            let paper_id = metadata
                .get("paperId")
                .and_then(|value| value.as_str())
                .or_else(|| fallback_ids.map(|(paper, _)| paper))
                .unwrap_or("");
            let section_id = metadata
                .get("sectionId")
                .and_then(|value| value.as_str())
                .or_else(|| fallback_ids.map(|(_, section)| section))
                .unwrap_or("");
            let catalog_titles =
                source_catalog.get(&(paper_id.to_string(), section_id.to_string()));
            let paper_title = catalog_titles
                .map(|(title, _)| title.as_str())
                .filter(|title| !title.is_empty())
                .unwrap_or(&article_title);
            let section_title = catalog_titles
                .map(|(_, title)| title.as_str())
                .filter(|title| !title.is_empty())
                .unwrap_or(section_id);
            source_evidence
                .entry(word)
                .or_default()
                .push(serde_json::json!({
                    "paperId": paper_id,
                    "paperTitle": paper_title,
                    "sectionId": section_id,
                    "sectionTitle": section_title,
                    "fuzzyMarkCount": fuzzy,
                    "familiarMarkCount": familiar,
                    "unknownMarkCount": unknown,
                    "wrongAssociationCount": wrong
                }));
        }
        drop(source_statement);

        let mut statement = conn
            .prepare(
                "SELECT normalized_form,
                        SUM(CASE WHEN mark_level = 'fuzzy' OR
                            (mark_level = 'none' AND user_mark = 'ignored') THEN 1 ELSE 0 END),
                        SUM(CASE WHEN mark_level = 'familiar' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN mark_level = 'unknown' OR
                            (mark_level = 'none' AND user_mark = 'unknown') THEN 1 ELSE 0 END),
                        SUM(CASE WHEN user_mark = 'wrong' THEN 1 ELSE 0 END),
                        SUM(CASE WHEN user_mark = 'mastered' THEN 1 ELSE 0 END),
                        MAX(updated_at)
                 FROM exercise_vocab_occurrences
                 GROUP BY normalized_form",
            )
            .map_err(|error| format!("Failed to prepare vocabulary evidence query: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            })
            .map_err(|error| format!("Failed to load vocabulary evidence: {error}"))?;
        let mut evidence = BTreeMap::new();
        for row in rows {
            let (word, fuzzy, familiar, unknown, wrong, mastered, last_marked_at) =
                row.map_err(|error| format!("Failed to read vocabulary evidence: {error}"))?;
            evidence.insert(
                word,
                (fuzzy, familiar, unknown, wrong, mastered, last_marked_at),
            );
        }

        let mut seen_words = BTreeSet::new();
        let mut items = frequencies
            .into_iter()
            .map(|frequency| {
                seen_words.insert(frequency.word.clone());
                let (fuzzy, familiar, unknown, wrong, mastered, last_marked_at) = evidence
                    .get(&frequency.word)
                    .cloned()
                    .unwrap_or((0, 0, 0, 0, 0, None));
                let score = frequency.paper_count as f64 * 4.0
                    + frequency.article_count as f64 * 1.5
                    + frequency.occurrence_count as f64 * 0.2
                    + fuzzy as f64 * 8.0
                    + familiar as f64 * 14.0
                    + unknown as f64 * 24.0
                    + wrong as f64 * 32.0
                    - mastered as f64 * 2.0;
                serde_json::json!({
                    "word": frequency.word,
                    "priorityScore": score,
                    "paperCount": frequency.paper_count,
                    "articleCount": frequency.article_count,
                    "occurrenceCount": frequency.occurrence_count,
                    "fuzzyMarkCount": fuzzy,
                    "familiarMarkCount": familiar,
                    "unknownMarkCount": unknown,
                    "wrongAssociationCount": wrong,
                    "masteredMarkCount": mastered,
                    "lastMarkedAt": last_marked_at,
                    "sources": source_evidence.get(&frequency.word).cloned().unwrap_or_default(),
                    "factors": [
                        {"type": "crossPaperFrequency", "value": frequency.paper_count},
                        {"type": "articleCoverage", "value": frequency.article_count},
                        {"type": "fuzzyMarks", "value": fuzzy},
                        {"type": "familiarMarks", "value": familiar},
                        {"type": "unknownMarks", "value": unknown},
                        {"type": "wrongAnswerAssociation", "value": wrong},
                        {"type": "masteryOffset", "value": mastered}
                    ]
                })
            })
            .collect::<Vec<_>>();
        for (word, (fuzzy, familiar, unknown, wrong, mastered, last_marked_at)) in &evidence {
            if seen_words.contains(word) || fuzzy + familiar + unknown + wrong == 0 {
                continue;
            }
            let score = *fuzzy as f64 * 8.0
                + *familiar as f64 * 14.0
                + *unknown as f64 * 24.0
                + *wrong as f64 * 32.0
                - *mastered as f64 * 2.0;
            items.push(serde_json::json!({
                "word": word,
                "priorityScore": score,
                "paperCount": 0,
                "articleCount": 0,
                "occurrenceCount": fuzzy + familiar + unknown + wrong + mastered,
                "fuzzyMarkCount": fuzzy,
                "familiarMarkCount": familiar,
                "unknownMarkCount": unknown,
                "wrongAssociationCount": wrong,
                "masteredMarkCount": mastered,
                "lastMarkedAt": last_marked_at,
                "sources": source_evidence.get(word).cloned().unwrap_or_default(),
                "factors": [
                    {"type": "fuzzyMarks", "value": fuzzy},
                    {"type": "familiarMarks", "value": familiar},
                    {"type": "unknownMarks", "value": unknown},
                    {"type": "wrongAnswerAssociation", "value": wrong},
                    {"type": "masteryOffset", "value": mastered}
                ]
            }));
        }
        items.sort_by(|left, right| {
            right["priorityScore"]
                .as_f64()
                .unwrap_or(0.0)
                .total_cmp(&left["priorityScore"].as_f64().unwrap_or(0.0))
        });
        serde_json::to_string(&serde_json::json!({
            "generatedAt": chrono::Utc::now().to_rfc3339(),
            "deterministic": true,
            "items": items
        }))
        .map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

pub fn get_exam_practice_report() -> Result<String, String> {
    with_runtime(|runtime, conn| {
        let asset_dir = exam_asset_dir(runtime);
        let mut papers = exam_practice_domain::load_all_exam_papers(&asset_dir)?;
        papers.extend(load_user_exam_papers(conn)?);
        papers.sort_by(|left, right| {
            left.year
                .cmp(&right.year)
                .then_with(|| left.month.cmp(&right.month))
                .then_with(|| left.set.cmp(&right.set))
                .then_with(|| left.title.cmp(&right.title))
        });

        let mut statement = conn
            .prepare(
                "SELECT paper_id, section_id,
                        SUM(CASE WHEN is_correct = 1 THEN 1 ELSE 0 END),
                        COUNT(*)
                 FROM exercise_attempts
                 WHERE is_correct IS NOT NULL
                   AND status IN ('answered', 'completed')
                 GROUP BY paper_id, section_id",
            )
            .map_err(|error| format!("Failed to prepare exam report query: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|error| format!("Failed to query exam report: {error}"))?;
        let mut attempts = BTreeMap::<(String, String), (i64, i64)>::new();
        for row in rows {
            let (paper_id, section_id, correct, total) =
                row.map_err(|error| format!("Failed to read exam report row: {error}"))?;
            attempts.insert((paper_id, section_id), (correct, total));
        }

        let mut by_exam = BTreeMap::<String, Vec<serde_json::Value>>::new();
        for paper in papers {
            let mut correct_count = 0i64;
            let mut total_questions = 0i64;
            let mut question_types = BTreeMap::<String, (i64, i64)>::new();
            for section in &paper.sections {
                let Some((correct, total)) = attempts.get(&(paper.id.clone(), section.id.clone()))
                else {
                    continue;
                };
                correct_count += correct;
                total_questions += total;
                if let Some(kind) =
                    exam_objective_section_kind(&section.section_type, &section.title)
                {
                    let entry = question_types.entry(kind.to_string()).or_insert((0, 0));
                    entry.0 += correct;
                    entry.1 += total;
                }
            }
            if total_questions == 0 {
                continue;
            }
            by_exam
                .entry(paper.exam.clone())
                .or_default()
                .push(serde_json::json!({
                    "paperId": paper.id,
                    "title": paper.title,
                    "year": paper.year,
                    "correctCount": correct_count,
                    "totalQuestions": total_questions,
                    "accuracyPercent": exam_accuracy_percent(correct_count, total_questions),
                    "questionTypes": question_types
                        .into_iter()
                        .map(|(kind, (correct, total))| {
                            (kind, serde_json::json!({
                                "correctCount": correct,
                                "totalQuestions": total,
                                "accuracyPercent": exam_accuracy_percent(correct, total)
                            }))
                        })
                        .collect::<BTreeMap<_, _>>()
                }));
        }

        serde_json::to_string(&serde_json::json!({
            "generatedAt": chrono::Utc::now().to_rfc3339(),
            "exams": by_exam
                .into_iter()
                .map(|(exam, papers)| serde_json::json!({
                    "exam": exam,
                    "papers": papers
                }))
                .collect::<Vec<_>>()
        }))
        .map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

fn exam_accuracy_percent(correct: i64, total: i64) -> f64 {
    if total <= 0 {
        0.0
    } else {
        correct as f64 / total as f64 * 100.0
    }
}

fn exam_objective_section_kind(section_type: &str, title: &str) -> Option<&'static str> {
    let haystack = format!("{} {}", section_type, title).to_lowercase();
    if haystack.contains("listening") || haystack.contains("听力") {
        return Some("listening");
    }
    if haystack.contains("cloze")
        || haystack.contains("完型")
        || haystack.contains("完形")
        || haystack.contains("选词填空")
    {
        return Some("cloze");
    }
    if haystack.contains("新题型") || haystack.contains("matching") {
        return Some("newType");
    }
    if haystack.contains("reading")
        || haystack.contains("阅读")
        || haystack.contains("passage")
        || section_type.to_lowercase().starts_with("text")
    {
        return Some("reading");
    }
    None
}

struct PreparedExamCausalAnalysis {
    evidence_key: String,
    evidence: Vec<serde_json::Value>,
    article_id: String,
    paper_id: String,
    question_id: String,
    attempt_id: String,
    prompt: String,
}

enum ExamCausalPreparation {
    Complete(String),
    Ready(PreparedExamCausalAnalysis),
}

const EXAM_CAUSAL_SYSTEM_PROMPT: &str = "Analyze every marked vocabulary item for this wrong answer, and include a candidate only when its contextual meaning plausibly caused confusion between the selected distractor and the correct choice. Use paragraphTranslations to determine the word's meaning in this passage before comparing the selected distractor with the reference answer. Return strict JSON only: {eligible:true,candidates:[{word,confidence,reasoning,evidence,limitations}],limitations:[string]}. Candidate words must come from markedVocabulary. Return at most two candidates. Keep every reasoning, evidence, and limitation string under 80 Simplified Chinese characters, and keep the whole response under 350 tokens. Every reasoning, evidence, and limitations string must use Simplified Chinese only; do not provide English explanations. Every string value must be valid JSON: do not use unescaped ASCII double-quote characters inside a string; use Chinese quotation marks instead.";

const EXAM_SECTION_CAUSAL_SYSTEM_PROMPT: &str = "Analyze every wrongQuestion independently using the shared passage, paragraphTranslations, and markedVocabulary. A candidate may be returned only when that marked word's contextual meaning plausibly caused confusion between that question's selected distractor and correct answer. Return strict JSON only: {eligible:true,questions:[{questionId,candidates:[{word,confidence,reasoning,evidence,limitations}]}],limitations:[string]}. Return one questions item for every supplied wrongQuestion, even when its candidates array is empty. Candidate words must come from markedVocabulary. Return at most two candidates per question. Keep every reasoning, evidence, and limitation string under 80 Simplified Chinese characters, and keep the whole response under 700 tokens. Every reasoning, evidence, and limitations string must use Simplified Chinese only; do not provide English explanations. Every string value must be valid JSON: do not use unescaped ASCII double-quote characters inside a string; use Chinese quotation marks instead.";

const EXAM_CLOZE_REVIEW_SYSTEM_PROMPT: &str = "Analyze the cloze using the supplied passage, markedVocabulary, questions, choices, and answers. The application owns all factual report fields; do not reproduce question numbers, stems, answer labels, option text or meanings, vocabulary meanings, marks, occurrence counts, ranks, or mark scope. Return strict JSON only: {eligible:true,questions:[{questionId,contextSentence,annotatedContext,optionAnalysis:[{label,analysis}],analysis,knowledgeGap,candidates:[{word,confidence,reasoning,evidence,limitations}]}],correctMarkedQuestions:[{questionId,distinction}],vocabularyPriority:[{word,priorityReason}],limitations:[string]}. Return one questions item per wrongQuestion. Explain how the marked words, context, selected distractor, and correct option interact. Candidate words and vocabularyPriority words must come from markedVocabulary. In annotatedContext gloss current-scope marks only and never gloss prior-scope pure-purple marks. Use concise Simplified Chinese and valid JSON.";

const EXAM_READING_REVIEW_SYSTEM_PROMPT: &str = "Analyze the reading questions using the supplied passage, markedVocabulary, choices, selected answers, and correct answers. The application owns all factual report fields; do not reproduce question numbers, stems, answer labels, option text or meanings, vocabulary meanings, marks, occurrence counts, ranks, or mark scope. Return strict JSON only: {eligible:true,questions:[{questionId,evidenceLocation,contextSentence,annotatedContext,optionAnalysis:[{label,analysis}],analysis,knowledgeGap,candidates:[{word,confidence,reasoning,evidence,limitations}]}],correctMarkedQuestions:[{questionId,distinction}],vocabularyPriority:[{word,priorityReason}],limitations:[string]}. Return one questions item per wrongQuestion. Work question-first and explain how the marked words, passage evidence, selected distractor, and correct option interact. Candidate words and vocabularyPriority words must come from markedVocabulary. In annotatedContext gloss current-scope marks only and never gloss prior-scope pure-purple marks. Use concise Simplified Chinese and valid JSON.";

fn build_exam_causal_prompt(
    section: &exam_practice_domain::ExamSection,
    question: &exam_practice_domain::ExamQuestion,
    selected_answer: Option<&str>,
    evidence: &[serde_json::Value],
) -> serde_json::Value {
    serde_json::json!({
        "passage": section.passage,
        "paragraphTranslations": section.paragraph_translations,
        "stem": question.stem,
        "choices": question.choices,
        "correctAnswer": question.answer,
        "selectedAnswer": selected_answer,
        "markedVocabulary": evidence
    })
}

fn build_exam_section_causal_prompt(
    section: &exam_practice_domain::ExamSection,
    wrong_questions: &[(exam_practice_domain::ExamQuestion, String)],
    evidence: &[serde_json::Value],
) -> serde_json::Value {
    serde_json::json!({
        "passage": section.passage,
        "paragraphTranslations": section.paragraph_translations,
        "markedVocabulary": evidence,
        "wrongQuestions": wrong_questions.iter().map(|(question, selected_answer)| serde_json::json!({
            "questionId": question.id,
            "number": question.number,
            "stem": question.stem,
            "choices": question.choices,
            "correctAnswer": question.answer,
            "selectedAnswer": selected_answer,
        })).collect::<Vec<_>>(),
    })
}

fn build_exam_cloze_review_prompt(
    section: &exam_practice_domain::ExamSection,
    attempts: &[(exam_practice_domain::ExamQuestion, String, bool)],
    evidence: &[serde_json::Value],
) -> serde_json::Value {
    let question_json = |question: &exam_practice_domain::ExamQuestion, selected_answer: &str| {
        serde_json::json!({
            "questionId": question.id,
            "number": question.number,
            "stem": question.stem,
            "choices": question.choices,
            "correctAnswer": question.answer,
            "selectedAnswer": selected_answer,
        })
    };
    serde_json::json!({
        "reviewFormat": "cloze-review-v1",
        "passage": section.passage,
        "paragraphTranslations": section.paragraph_translations,
        "markedVocabulary": evidence,
        "wrongQuestions": attempts.iter()
            .filter(|(_, _, is_correct)| !is_correct)
            .map(|(question, selected_answer, _)| question_json(question, selected_answer))
            .collect::<Vec<_>>(),
        "correctQuestions": attempts.iter()
            .filter(|(question, _, is_correct)| {
                *is_correct && question_choices_contain_marked_evidence(question, evidence)
            })
            .map(|(question, selected_answer, _)| question_json(question, selected_answer))
            .collect::<Vec<_>>(),
    })
}

fn build_exam_reading_review_prompt(
    section: &exam_practice_domain::ExamSection,
    attempts: &[(exam_practice_domain::ExamQuestion, String, bool)],
    evidence: &[serde_json::Value],
) -> serde_json::Value {
    let question_json = |question: &exam_practice_domain::ExamQuestion, selected_answer: &str| {
        serde_json::json!({
            "questionId": question.id,
            "number": question.number,
            "stem": question.stem,
            "choices": question.choices,
            "correctAnswer": question.answer,
            "selectedAnswer": selected_answer,
        })
    };
    serde_json::json!({
        "reviewFormat": "reading-review-v1",
        "passage": section.passage,
        "paragraphTranslations": section.paragraph_translations,
        "markedVocabulary": evidence,
        "wrongQuestions": attempts.iter()
            .filter(|(_, _, is_correct)| !is_correct)
            .map(|(question, selected_answer, _)| question_json(question, selected_answer))
            .collect::<Vec<_>>(),
        "correctQuestions": attempts.iter()
            .filter(|(question, _, is_correct)| {
                *is_correct && question_choices_contain_marked_evidence(question, evidence)
            })
            .map(|(question, selected_answer, _)| question_json(question, selected_answer))
            .collect::<Vec<_>>(),
    })
}

fn question_choices_contain_marked_evidence(
    question: &exam_practice_domain::ExamQuestion,
    evidence: &[serde_json::Value],
) -> bool {
    let choices = question
        .choices
        .iter()
        .map(|choice| choice.text.to_ascii_lowercase())
        .collect::<Vec<_>>();
    evidence.iter().any(|item| {
        let word = item
            .get("word")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        !word.is_empty() && choices.iter().any(|choice| choice.contains(&word))
    })
}

struct PreparedExamSectionQuestion {
    question_id: String,
    question_number: i64,
    stem: String,
    choices: Vec<serde_json::Value>,
    correct_answer: String,
    attempt_id: String,
    selected_answer: String,
}

struct PreparedExamSectionCausalAnalysis {
    article_id: String,
    paper_id: String,
    evidence: Vec<serde_json::Value>,
    questions: Vec<PreparedExamSectionQuestion>,
    correct_question_numbers: std::collections::HashMap<String, i64>,
    is_cloze: bool,
    is_reading: bool,
    prompt: String,
}

enum ExamSectionCausalPreparation {
    Complete(String),
    Ready(PreparedExamSectionCausalAnalysis),
}

pub fn analyze_exam_section_vocabulary(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let exam = required_string_field(&request, "exam")?;
    let paper_id = required_string_field(&request, "paperId")?;
    let section_id = required_string_field(&request, "sectionId")?;
    let attempts = request
        .get("attempts")
        .and_then(serde_json::Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "Missing attempts".to_string())?;
    let pure_purple_words = request
        .get("purePurpleWords")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(|word| word.trim().to_ascii_lowercase())
        .filter(|word| !word.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let preparation = with_runtime(|runtime, conn| {
        let paper = if let Some(paper) = load_user_exam_paper(conn, &exam, &paper_id)? {
            paper
        } else {
            let asset_dir = exam_asset_dir(runtime);
            exam_practice_domain::load_exam_paper(&asset_dir, &exam, &paper_id)?
                .ok_or_else(|| format!("Exam paper not found: {paper_id}"))?
        };
        let section = paper
            .sections
            .iter()
            .find(|section| section.id == section_id)
            .ok_or_else(|| format!("Exam section not found: {section_id}"))?;
        let is_cloze =
            exam_objective_section_kind(&section.section_type, &section.title) == Some("cloze");
        let is_reading =
            exam_objective_section_kind(&section.section_type, &section.title) == Some("reading");
        let mut wrong_questions = Vec::new();
        let mut completed_questions = Vec::new();
        let mut prepared_questions = Vec::new();
        let mut missing_evidence = Vec::new();
        for item in attempts {
            let question_id = required_string_field(item, "questionId")?;
            let attempt_id = required_string_field(item, "attemptId")?;
            let question = section
                .questions
                .iter()
                .find(|question| question.id == question_id)
                .ok_or_else(|| format!("Exam question not found: {question_id}"))?;
            let attempt =
                word_storage_core::persistence::exercise_vocab_repo::get_exercise_attempt(
                    conn,
                    &attempt_id,
                )
                .map_err(|error| format!("Failed to load exam attempt: {error}"))?;
            let Some(attempt) = attempt else {
                missing_evidence.push(format!("Question {question_id} has no recorded attempt"));
                continue;
            };
            if !question.capabilities.causal_analyzable || attempt.is_correct.is_none() {
                missing_evidence.push(format!(
                    "Question {question_id} is not an analyzable answer"
                ));
                continue;
            }
            let selected_answer = attempt.selected_answer.unwrap_or_default();
            let is_correct = attempt.is_correct == Some(true);
            if !is_correct {
                wrong_questions.push((question.clone(), selected_answer.clone()));
                prepared_questions.push(PreparedExamSectionQuestion {
                    question_id,
                    question_number: question.number,
                    stem: question.stem.clone(),
                    choices: question
                        .choices
                        .iter()
                        .map(|choice| {
                            serde_json::json!({
                                "label": choice.label,
                                "text": choice.text,
                            })
                        })
                        .collect(),
                    correct_answer: question.answer.clone().unwrap_or_default(),
                    attempt_id,
                    selected_answer: selected_answer.clone(),
                });
            } else if !is_cloze && !is_reading {
                missing_evidence.push(format!(
                    "Question {question_id} is not a wrong analyzable answer"
                ));
                continue;
            }
            completed_questions.push((question.clone(), selected_answer, is_correct));
        }
        if wrong_questions.is_empty() {
            return Ok(ExamSectionCausalPreparation::Complete(
                serde_json::json!({
                    "eligible": false,
                    "missingEvidence": missing_evidence,
                    "questions": []
                })
                .to_string(),
            ));
        }
        let article_id = format!("{paper_id}:{section_id}");
        let mut evidence = load_exam_causal_evidence(conn, &article_id)?;
        augment_pure_purple_exam_evidence(conn, &article_id, &pure_purple_words, &mut evidence)?;
        enrich_exam_derivational_family_evidence(
            &runtime.paths().bundled_resource_path(""),
            &mut evidence,
        )?;
        if evidence.is_empty() {
            return Ok(ExamSectionCausalPreparation::Complete(
                serde_json::json!({
                    "eligible": false,
                    "missingEvidence": ["No marked vocabulary is available for this article"],
                    "questions": []
                })
                .to_string(),
            ));
        }
        let prompt = if is_cloze {
            build_exam_cloze_review_prompt(section, &completed_questions, &evidence)
        } else if is_reading {
            build_exam_reading_review_prompt(section, &completed_questions, &evidence)
        } else {
            build_exam_section_causal_prompt(section, &wrong_questions, &evidence)
        };
        let correct_question_numbers = completed_questions
            .iter()
            .filter(|(question, _, is_correct)| {
                *is_correct && question_choices_contain_marked_evidence(question, &evidence)
            })
            .map(|(question, _, _)| (question.id.clone(), question.number))
            .collect();
        Ok(ExamSectionCausalPreparation::Ready(
            PreparedExamSectionCausalAnalysis {
                article_id,
                paper_id,
                evidence,
                questions: prepared_questions,
                correct_question_numbers,
                is_cloze,
                is_reading,
                prompt: prompt.to_string(),
            },
        ))
    })?;
    let prepared = match preparation {
        ExamSectionCausalPreparation::Complete(result) => return Ok(result),
        ExamSectionCausalPreparation::Ready(prepared) => prepared,
    };

    // One provider request owns the shared article context for every wrong question.
    let system_prompt = if prepared.is_cloze {
        EXAM_CLOZE_REVIEW_SYSTEM_PROMPT
    } else if prepared.is_reading {
        EXAM_READING_REVIEW_SYSTEM_PROMPT
    } else {
        EXAM_SECTION_CAUSAL_SYSTEM_PROMPT
    };
    let mut result = normalize_exam_section_causal_provider_result(
        ai_agent().run_exam_summary_json(system_prompt, &prepared.prompt),
    );
    retain_marked_exam_section_candidates(&mut result, &prepared.evidence, &prepared.questions);
    if prepared.is_cloze || prepared.is_reading {
        attach_local_structured_review_facts(&mut result, &prepared.questions);
        remove_familiar_inline_glosses(&mut result, &prepared.evidence);
        retain_cloze_correct_question_reviews(&mut result, &prepared.correct_question_numbers);
        result["reviewFormat"] = serde_json::Value::String(
            if prepared.is_cloze {
                "cloze-review-v1"
            } else {
                "reading-review-v1"
            }
            .to_string(),
        );
        attach_cloze_vocabulary_priority(&mut result, &prepared.evidence);
    }
    let status = if result["providerStatus"] == "completed" {
        "completed"
    } else {
        "provider_failure"
    };
    with_runtime_conn(|conn| {
        let candidates = result
            .get("questions")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|question| {
                question
                    .get("candidates")
                    .and_then(serde_json::Value::as_array)
            })
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        let promoted_words = persist_exam_causal_words(
            conn,
            &prepared.article_id,
            &serde_json::json!({"candidates": candidates}),
        )?;
        result["promotedWords"] = serde_json::json!(promoted_words);
        for question in &prepared.questions {
            let question_result = result
                .get("questions")
                .and_then(serde_json::Value::as_array)
                .and_then(|items| {
                    items
                        .iter()
                        .find(|item| item["questionId"] == question.question_id)
                })
                .cloned()
                .unwrap_or_else(
                    || serde_json::json!({"questionId": question.question_id, "candidates": []}),
                );
            let evidence_key = format!(
                "{}:{}:{}:{}:section-batch-v1",
                prepared.paper_id,
                question.question_id,
                question.attempt_id,
                question.selected_answer
            );
            persist_exam_analysis(
                conn,
                &evidence_key,
                &prepared.paper_id,
                &question.question_id,
                &question.attempt_id,
                status,
                serde_json::json!({
                    "eligible": result["eligible"],
                    "providerStatus": result["providerStatus"],
                    "candidates": question_result["candidates"],
                    "limitations": result["limitations"],
                    "promotedWords": result["promotedWords"],
                }),
            )?;
        }
        Ok(result.to_string())
    })
}

pub fn analyze_exam_question_vocabulary(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let exam = required_string_field(&request, "exam")?;
    let paper_id = required_string_field(&request, "paperId")?;
    let section_id = required_string_field(&request, "sectionId")?;
    let question_id = required_string_field(&request, "questionId")?;
    let attempt_id = required_string_field(&request, "attemptId")?;
    let preparation = with_runtime(|runtime, conn| {
        let paper = if let Some(paper) = load_user_exam_paper(conn, &exam, &paper_id)? {
            paper
        } else {
            let asset_dir = exam_asset_dir(runtime);
            exam_practice_domain::load_exam_paper(&asset_dir, &exam, &paper_id)?
                .ok_or_else(|| format!("Exam paper not found: {paper_id}"))?
        };
        let section = paper
            .sections
            .iter()
            .find(|section| section.id == section_id)
            .ok_or_else(|| format!("Exam section not found: {section_id}"))?;
        let question = section
            .questions
            .iter()
            .find(|question| question.id == question_id)
            .ok_or_else(|| format!("Exam question not found: {question_id}"))?;
        let attempt = word_storage_core::persistence::exercise_vocab_repo::get_exercise_attempt(
            conn,
            &attempt_id,
        )
        .map_err(|error| format!("Failed to load exam attempt: {error}"))?;
        let missing = if !question.capabilities.causal_analyzable {
            Some("Question does not have an authoritative answer and analyzable reading context")
        } else if attempt.as_ref().and_then(|item| item.is_correct) != Some(false) {
            Some("A recorded wrong answer is required before causal analysis")
        } else {
            None
        };
        if let Some(reason) = missing {
            return persist_exam_analysis(
                conn,
                &format!("{paper_id}:{question_id}:{attempt_id}:ineligible"),
                &paper_id,
                &question_id,
                &attempt_id,
                "ineligible",
                serde_json::json!({"eligible": false, "missingEvidence": [reason]}),
            )
            .map(ExamCausalPreparation::Complete);
        }
        let article_id = format!("{paper_id}:{section_id}");
        let mut evidence = load_exam_causal_evidence(conn, &article_id)?;
        enrich_exam_derivational_family_evidence(
            &runtime.paths().bundled_resource_path(""),
            &mut evidence,
        )?;
        if evidence.is_empty() {
            return persist_exam_analysis(
                conn,
                &format!("{paper_id}:{question_id}:{attempt_id}:no-marked-vocabulary"),
                &paper_id,
                &question_id,
                &attempt_id,
                "ineligible",
                serde_json::json!({
                    "eligible": false,
                    "missingEvidence": ["本篇文章尚未标记需要分析的词汇"]
                }),
            )
            .map(ExamCausalPreparation::Complete);
        }
        let attempt = attempt.expect("wrong attempt checked");
        let evidence_key = format!(
            "{paper_id}:{question_id}:{attempt_id}:{}:{}",
            attempt.selected_answer.as_deref().unwrap_or(""),
            evidence
                .iter()
                .filter_map(|item| item.get("word").and_then(|word| word.as_str()))
                .collect::<Vec<_>>()
                .join(",")
        );
        if let Some(cached) = conn
            .query_row(
                "SELECT result_json FROM exam_question_analyses WHERE evidence_key = ?1",
                [&evidence_key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Failed to load cached exam analysis: {error}"))?
        {
            let mut cached_result = serde_json::from_str::<serde_json::Value>(&cached)
                .map_err(|error| format!("Cached exam analysis is invalid: {error}"))?;
            retain_marked_exam_causal_candidates(&mut cached_result, &evidence);
            let promoted_words = persist_exam_causal_words(conn, &article_id, &cached_result)?;
            cached_result["promotedWords"] = serde_json::json!(promoted_words);
            return Ok(ExamCausalPreparation::Complete(cached_result.to_string()));
        }
        let prompt = build_exam_causal_prompt(
            section,
            question,
            attempt.selected_answer.as_deref(),
            &evidence,
        );
        Ok(ExamCausalPreparation::Ready(PreparedExamCausalAnalysis {
            evidence_key,
            evidence,
            article_id,
            paper_id: paper_id.clone(),
            question_id: question_id.clone(),
            attempt_id: attempt_id.clone(),
            prompt: prompt.to_string(),
        }))
    })?;
    let prepared = match preparation {
        ExamCausalPreparation::Complete(result) => return Ok(result),
        ExamCausalPreparation::Ready(prepared) => prepared,
    };

    // Provider I/O must never hold the runtime guard or a SQLite connection.
    let mut result = normalize_exam_causal_provider_result(
        ai_agent().run_exam_summary_json(EXAM_CAUSAL_SYSTEM_PROMPT, &prepared.prompt),
    );
    retain_marked_exam_causal_candidates(&mut result, &prepared.evidence);
    let status = if result["providerStatus"] == "completed" {
        "completed"
    } else {
        "provider_failure"
    };
    with_runtime_conn(|conn| {
        let promoted_words = persist_exam_causal_words(conn, &prepared.article_id, &result)?;
        let mut result = result;
        result["promotedWords"] = serde_json::json!(promoted_words);
        persist_exam_analysis(
            conn,
            &prepared.evidence_key,
            &prepared.paper_id,
            &prepared.question_id,
            &prepared.attempt_id,
            status,
            result,
        )
    })
}

fn load_exam_causal_evidence(
    conn: &rusqlite::Connection,
    article_source_id: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let mut statement = conn
        .prepare(
            "SELECT o.normalized_form, o.mark_level, o.sentence_text,
                    COALESCE(NULLIF(o.meaning_note, ''), ''),
                    COALESCE(e.exam_frequency, 0), e.exam_rank
             FROM exercise_vocab_occurrences o
             JOIN exercise_articles a ON a.id = o.article_id
             LEFT JOIN entries e ON e.id = o.entry_id
             WHERE a.article_id = ?1
               AND (o.mark_level IN ('fuzzy', 'familiar', 'unknown')
                    OR o.user_mark IN ('unknown', 'wrong'))
             ORDER BY o.start_offset",
        )
        .map_err(|error| format!("Failed to prepare question evidence query: {error}"))?;
    let rows = statement
        .query_map([article_source_id], |row| {
            let mark_level = row.get::<_, String>(1)?;
            Ok(serde_json::json!({
                "word": row.get::<_, String>(0)?,
                "mark": mark_level,
                "markScope": "current",
                "sentence": row.get::<_, String>(2)?,
                "meaning": row.get::<_, String>(3)?,
                "examFrequency": row.get::<_, i64>(4)?,
                "examRank": row.get::<_, Option<i64>>(5)?
            }))
        })
        .map_err(|error| format!("Failed to load question vocabulary evidence: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to read question vocabulary evidence: {error}"))
}

fn augment_pure_purple_exam_evidence(
    conn: &rusqlite::Connection,
    article_source_id: &str,
    pure_purple_words: &std::collections::HashSet<String>,
    evidence: &mut Vec<serde_json::Value>,
) -> Result<(), String> {
    for word in pure_purple_words {
        if let Some(item) = evidence.iter_mut().find(|item| {
            item.get("word")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|value| value.eq_ignore_ascii_case(word))
        }) {
            item["markScope"] = serde_json::Value::String("prior".to_string());
            continue;
        }
        let prior = conn
            .query_row(
                "SELECT o.normalized_form, o.mark_level,
                        COALESCE(NULLIF(o.meaning_note, ''), ''),
                        COALESCE(e.exam_frequency, 0), e.exam_rank
                 FROM exercise_vocab_occurrences o
                 JOIN exercise_articles a ON a.id = o.article_id
                 LEFT JOIN entries e ON e.id = o.entry_id
                 WHERE a.article_id <> ?1
                   AND LOWER(o.normalized_form) = ?2
                   AND (o.mark_level IN ('fuzzy', 'familiar', 'unknown')
                        OR o.user_mark IN ('unknown', 'wrong'))
                 ORDER BY o.updated_at DESC
                 LIMIT 1",
                rusqlite::params![article_source_id, word],
                |row| {
                    Ok(serde_json::json!({
                        "word": row.get::<_, String>(0)?,
                        "mark": row.get::<_, String>(1)?,
                        "markScope": "prior",
                        "sentence": "",
                        "meaning": row.get::<_, String>(2)?,
                        "examFrequency": row.get::<_, i64>(3)?,
                        "examRank": row.get::<_, Option<i64>>(4)?
                    }))
                },
            )
            .optional()
            .map_err(|error| format!("Failed to load prior exam vocabulary evidence: {error}"))?;
        if let Some(prior) = prior {
            evidence.push(prior);
        }
    }
    Ok(())
}

fn parse_exam_derivational_family_index(
    source: &str,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let entries: serde_json::Value = serde_json::from_str(source)
        .map_err(|error| format!("Failed to parse Kaoyan derivational families: {error}"))?;
    let mut index = BTreeMap::new();
    if let Some(families) = entries.as_object() {
        for (member, family) in families {
            insert_exam_derivational_family(
                &mut index,
                member,
                family,
                family
                    .get("strictOccurrences")
                    .and_then(serde_json::Value::as_i64),
            );
        }
        return Ok(index);
    }
    for entry in entries.as_array().into_iter().flatten() {
        let Some(family) =
            entry.pointer("/content/word/content/realExamFrequency/derivationalFamily")
        else {
            continue;
        };
        let Some(members) = family.get("members").and_then(serde_json::Value::as_object) else {
            continue;
        };
        for (member, strict_count) in members {
            insert_exam_derivational_family(&mut index, member, family, strict_count.as_i64());
        }
    }
    Ok(index)
}

fn insert_exam_derivational_family(
    index: &mut BTreeMap<String, serde_json::Value>,
    member: &str,
    family: &serde_json::Value,
    strict_count: Option<i64>,
) {
    let normalized = member.trim().to_ascii_lowercase();
    let root = family
        .get("root")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let occurrences = family
        .get("occurrences")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let Some(members) = family.get("members").and_then(serde_json::Value::as_object) else {
        return;
    };
    if normalized.is_empty() || root.is_empty() || occurrences <= 0 {
        return;
    }
    index.insert(
        normalized,
        serde_json::json!({
            "examFrequency": strict_count.unwrap_or(0),
            "examFamilyRoot": root,
            "examFamilyFrequency": occurrences,
            "examFamilyMembers": members,
        }),
    );
}

fn load_exam_derivational_family_index(
    bundle_dir: &Path,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let candidates = [
        bundle_dir
            .join("exam-dictionary")
            .join("kaoyan-derivational-frequencies.json"),
        bundle_dir
            .join("flutter_assets")
            .join("assets")
            .join("exam-dictionary")
            .join("kaoyan-derivational-frequencies.json"),
        bundle_dir
            .join("Frameworks")
            .join("App.framework")
            .join("flutter_assets")
            .join("assets")
            .join("exam-dictionary")
            .join("kaoyan-derivational-frequencies.json"),
        bundle_dir
            .join("seed-vocab")
            .join("book")
            .join("KaoYan_3.json"),
    ];
    let Some(path) = candidates.into_iter().find(|candidate| candidate.is_file()) else {
        return Ok(BTreeMap::new());
    };
    let cache_key = path.to_string_lossy().to_string();
    if let Some(cached) = EXAM_DERIVATIONAL_FAMILY_CACHE
        .lock()
        .map_err(|_| "Exam derivational family cache lock poisoned".to_string())?
        .get(&cache_key)
        .cloned()
    {
        return Ok(cached);
    }
    let source = fs::read_to_string(&path).map_err(|error| {
        format!(
            "Failed to read Kaoyan derivational families from {}: {error}",
            path.display()
        )
    })?;
    let index = parse_exam_derivational_family_index(&source)?;
    EXAM_DERIVATIONAL_FAMILY_CACHE
        .lock()
        .map_err(|_| "Exam derivational family cache lock poisoned".to_string())?
        .insert(cache_key, index.clone());
    Ok(index)
}

fn enrich_exam_derivational_family_evidence(
    bundle_dir: &Path,
    evidence: &mut [serde_json::Value],
) -> Result<(), String> {
    let index = load_exam_derivational_family_index(bundle_dir)?;
    for item in evidence {
        let word = item
            .get("word")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let Some(family) = index.get(&word) else {
            continue;
        };
        for key in [
            "examFrequency",
            "examFamilyRoot",
            "examFamilyFrequency",
            "examFamilyMembers",
        ] {
            item[key] = family.get(key).cloned().unwrap_or(serde_json::Value::Null);
        }
    }
    Ok(())
}

fn attach_cloze_vocabulary_priority(
    result: &mut serde_json::Value,
    evidence: &[serde_json::Value],
) {
    let reasons = result
        .get("vocabularyPriority")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            let word = item.get("word")?.as_str()?.trim().to_ascii_lowercase();
            let reason = item
                .get("priorityReason")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            Some((word, reason))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let mut by_word = std::collections::HashMap::<String, serde_json::Value>::new();
    for item in evidence {
        let word = item
            .get("word")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if word.is_empty() {
            continue;
        }
        let frequency = item
            .get("examFrequency")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0);
        let display_frequency = item
            .get("examFamilyFrequency")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(frequency);
        let replace = by_word
            .get(&word)
            .and_then(|current| current.get("displayExamFrequency"))
            .and_then(serde_json::Value::as_i64)
            .is_none_or(|current| display_frequency > current);
        if replace {
            by_word.insert(
                word.clone(),
                serde_json::json!({
                    "word": word,
                    "meaning": item.get("meaning").cloned().unwrap_or_default(),
                    "mark": item.get("mark").cloned().unwrap_or_default(),
                    "markScope": item.get("markScope").cloned().unwrap_or_default(),
                    "examFrequency": frequency,
                    "displayExamFrequency": display_frequency,
                    "examFamilyRoot": item.get("examFamilyRoot").cloned().unwrap_or_default(),
                    "examFamilyFrequency": item.get("examFamilyFrequency").cloned().unwrap_or_default(),
                    "examFamilyMembers": item.get("examFamilyMembers").cloned().unwrap_or_default(),
                    "examRank": item.get("examRank").cloned().unwrap_or_default(),
                    "priorityReason": reasons.get(&word).cloned().unwrap_or_default(),
                }),
            );
        }
    }
    let mut items = by_word.into_values().collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right["displayExamFrequency"]
            .as_i64()
            .unwrap_or(0)
            .cmp(&left["displayExamFrequency"].as_i64().unwrap_or(0))
            .then_with(|| {
                left["examRank"]
                    .as_i64()
                    .unwrap_or(i64::MAX)
                    .cmp(&right["examRank"].as_i64().unwrap_or(i64::MAX))
            })
            .then_with(|| left["word"].as_str().cmp(&right["word"].as_str()))
    });
    for (index, item) in items.iter_mut().enumerate() {
        item["priority"] = serde_json::json!(index + 1);
    }
    result["vocabularyPriority"] = serde_json::Value::Array(items);
}

fn attach_local_structured_review_facts(
    result: &mut serde_json::Value,
    questions: &[PreparedExamSectionQuestion],
) {
    let provider_questions = result
        .get("questions")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let local_questions = questions
        .iter()
        .map(|question| {
            let provider = provider_questions.iter().find(|item| {
                item.get("questionId").and_then(serde_json::Value::as_str)
                    == Some(question.question_id.as_str())
            });
            let provider_options = provider
                .and_then(|item| item.get("optionAnalysis"))
                .and_then(serde_json::Value::as_array);
            let option_analysis = question
                .choices
                .iter()
                .filter_map(|choice| {
                    let label = choice.get("label")?.as_str().unwrap_or("");
                    let text = choice.get("text")?.as_str().unwrap_or("");
                    let analysis = provider_options
                        .and_then(|items| {
                            items.iter().find(|item| {
                                item.get("label").and_then(serde_json::Value::as_str) == Some(label)
                            })
                        })
                        .and_then(|item| item.get("analysis"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    Some(serde_json::json!({
                        "label": label,
                        "meaning": text,
                        "analysis": analysis,
                    }))
                })
                .collect::<Vec<_>>();
            let ai_string = |key: &str| {
                provider
                    .and_then(|item| item.get(key))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("")
            };
            serde_json::json!({
                "questionId": question.question_id,
                "questionNumber": question.question_number,
                "stem": question.stem,
                "selectedAnswer": question.selected_answer,
                "correctAnswer": question.correct_answer,
                "evidenceLocation": ai_string("evidenceLocation"),
                "contextSentence": ai_string("contextSentence"),
                "annotatedContext": ai_string("annotatedContext"),
                "optionAnalysis": option_analysis,
                "analysis": ai_string("analysis"),
                "knowledgeGap": ai_string("knowledgeGap"),
                "candidates": provider
                    .and_then(|item| item.get("candidates"))
                    .and_then(serde_json::Value::as_array)
                    .cloned()
                    .unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    result["questions"] = serde_json::Value::Array(local_questions);
}

fn remove_familiar_inline_glosses(result: &mut serde_json::Value, evidence: &[serde_json::Value]) {
    let pure_purple_words = evidence
        .iter()
        .filter(|item| item.get("markScope").and_then(serde_json::Value::as_str) == Some("prior"))
        .filter_map(|item| item.get("word").and_then(serde_json::Value::as_str))
        .map(|word| word.trim().to_ascii_lowercase())
        .filter(|word| !word.is_empty())
        .collect::<std::collections::HashSet<_>>();
    for question in result
        .get_mut("questions")
        .and_then(serde_json::Value::as_array_mut)
        .into_iter()
        .flatten()
    {
        let Some(text) = question
            .get("annotatedContext")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let mut sanitized = text.to_string();
        for word in &pure_purple_words {
            let mut cursor = 0;
            while cursor < sanitized.len() {
                let lowered = sanitized.to_ascii_lowercase();
                let Some(relative_start) = lowered[cursor..].find(word) else {
                    break;
                };
                let start = cursor + relative_start;
                let after_word = start + word.len();
                let suffix = &sanitized[after_word..];
                let closing = if suffix.starts_with('（') {
                    suffix.find('）').map(|index| index + '）'.len_utf8())
                } else if suffix.starts_with('(') {
                    suffix.find(')').map(|index| index + 1)
                } else {
                    None
                };
                let Some(gloss_len) = closing else {
                    cursor = after_word;
                    continue;
                };
                sanitized.replace_range(after_word..after_word + gloss_len, "");
                cursor = after_word;
            }
        }
        question["annotatedContext"] = serde_json::Value::String(sanitized);
    }
}

fn retain_cloze_correct_question_reviews(
    result: &mut serde_json::Value,
    correct_question_numbers: &std::collections::HashMap<String, i64>,
) {
    let Some(items) = result
        .get_mut("correctMarkedQuestions")
        .and_then(serde_json::Value::as_array_mut)
    else {
        result["correctMarkedQuestions"] = serde_json::json!([]);
        return;
    };
    items.retain_mut(|item| {
        let Some(question_id) = item.get("questionId").and_then(serde_json::Value::as_str) else {
            return false;
        };
        let Some(number) = correct_question_numbers.get(question_id) else {
            return false;
        };
        item["questionNumber"] = serde_json::json!(number);
        item.get("distinction")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
    });
}

fn retain_marked_exam_causal_candidates(
    result: &mut serde_json::Value,
    evidence: &[serde_json::Value],
) {
    let allowed: std::collections::HashSet<String> = evidence
        .iter()
        .filter_map(|item| item.get("word").and_then(serde_json::Value::as_str))
        .map(|word| word.trim().to_ascii_lowercase())
        .filter(|word| !word.is_empty())
        .collect();
    let Some(candidates) = result
        .get_mut("candidates")
        .and_then(serde_json::Value::as_array_mut)
    else {
        result["candidates"] = serde_json::json!([]);
        return;
    };
    candidates.retain_mut(|candidate| {
        let word = candidate
            .get("word")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if !allowed.contains(&word) {
            return false;
        }
        let reasoning = candidate
            .get("reasoning")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if !reasoning
            .chars()
            .any(|value| ('\u{4e00}'..='\u{9fff}').contains(&value))
        {
            candidate["reasoning"] = serde_json::Value::String(format!(
                "标记词“{word}”可能影响了对题干、正确选项与所选干扰项之间语义关系的判断。"
            ));
        }
        true
    });
}

fn persist_exam_causal_words(
    conn: &rusqlite::Connection,
    article_source_id: &str,
    result: &serde_json::Value,
) -> Result<Vec<String>, String> {
    let mut promoted = Vec::new();
    for candidate in result
        .get("candidates")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let word = candidate
            .get("word")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if word.is_empty() || promoted.contains(&word) {
            continue;
        }
        let changed = conn
            .execute(
                "UPDATE exercise_vocab_occurrences
                 SET user_mark = 'wrong', updated_at = datetime('now')
                 WHERE article_id = (
                     SELECT id FROM exercise_articles WHERE article_id = ?1
                 ) AND LOWER(normalized_form) = LOWER(?2)",
                rusqlite::params![article_source_id, word],
            )
            .map_err(|error| format!("Failed to persist causal vocabulary: {error}"))?;
        if changed > 0 {
            promoted.push(word);
        }
    }
    Ok(promoted)
}

const EXAM_ANALYSIS_TASKS_KEY: &str = "exam_analysis_tasks_json";

pub fn save_exam_analysis_task(request_json: String) -> Result<String, String> {
    let task: serde_json::Value = serde_json::from_str(&request_json)
        .map_err(|error| format!("Invalid exam analysis task: {error}"))?;
    with_runtime_conn(|conn| {
        let saved = upsert_exam_analysis_task_with_connection(conn, task)?;
        serde_json::to_string(&saved).map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

pub fn get_exam_analysis_tasks() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let inbox = list_exam_analysis_tasks_with_connection(conn)?;
        serde_json::to_string(&inbox).map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

pub fn mark_exam_analysis_tasks_read() -> Result<String, String> {
    with_runtime_conn(|conn| {
        mark_exam_analysis_tasks_read_with_connection(conn)?;
        let inbox = list_exam_analysis_tasks_with_connection(conn)?;
        serde_json::to_string(&inbox).map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

fn upsert_exam_analysis_task_with_connection(
    conn: &word_storage_core::Connection,
    mut task: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let task_id = required_string_field(&task, "taskId")?;
    let status = required_string_field(&task, "status")?;
    if !matches!(status.as_str(), "running" | "completed" | "failed") {
        return Err(format!("Unsupported exam analysis task status: {status}"));
    }
    let stored = get_json_setting(conn, EXAM_ANALYSIS_TASKS_KEY, &serde_json::json!([]))?;
    let mut tasks = stored.as_array().cloned().unwrap_or_default();
    let previous = tasks
        .iter()
        .find(|item| item["taskId"].as_str() == Some(task_id.as_str()))
        .cloned();
    tasks.retain(|item| item["taskId"].as_str() != Some(task_id.as_str()));
    if let Some(scope_key) = exam_analysis_task_scope_key(&task) {
        tasks.retain(|item| exam_analysis_task_scope_key(item).as_deref() != Some(&scope_key));
    }
    let became_completed = status == "completed"
        && previous.as_ref().and_then(|item| item["status"].as_str()) != Some("completed");
    task["unread"] = serde_json::Value::Bool(if status == "completed" {
        became_completed
            || previous
                .as_ref()
                .and_then(|item| item["unread"].as_bool())
                .unwrap_or(false)
    } else {
        false
    });
    tasks.push(task.clone());
    tasks.sort_by(|left, right| {
        right["createdAt"]
            .as_str()
            .unwrap_or("")
            .cmp(left["createdAt"].as_str().unwrap_or(""))
    });
    tasks.truncate(50);
    set_json_setting(
        conn,
        EXAM_ANALYSIS_TASKS_KEY,
        &serde_json::Value::Array(tasks),
    )?;
    Ok(task)
}

fn list_exam_analysis_tasks_with_connection(
    conn: &word_storage_core::Connection,
) -> Result<serde_json::Value, String> {
    let stored = get_json_setting(conn, EXAM_ANALYSIS_TASKS_KEY, &serde_json::json!([]))?;
    let mut seen_scopes = std::collections::HashSet::new();
    let items = stored
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|item| exam_analysis_task_scope_key(item).is_none_or(|key| seen_scopes.insert(key)))
        .collect::<Vec<_>>();
    let unread_count = items
        .iter()
        .filter(|item| item["status"] == "completed" && item["unread"] == true)
        .count();
    Ok(serde_json::json!({
        "items": items,
        "unreadCount": unread_count
    }))
}

fn exam_analysis_task_scope_key(task: &serde_json::Value) -> Option<String> {
    let exam = task.get("exam")?.as_str()?.trim();
    let paper_id = task.get("paperId")?.as_str()?.trim();
    let section_id = task.get("sectionId")?.as_str()?.trim();
    if exam.is_empty() || paper_id.is_empty() || section_id.is_empty() {
        return None;
    }
    Some(format!("{exam}:{paper_id}:{section_id}"))
}

fn mark_exam_analysis_tasks_read_with_connection(
    conn: &word_storage_core::Connection,
) -> Result<(), String> {
    let stored = get_json_setting(conn, EXAM_ANALYSIS_TASKS_KEY, &serde_json::json!([]))?;
    let mut items = stored.as_array().cloned().unwrap_or_default();
    for item in &mut items {
        if item["status"] == "completed" {
            item["unread"] = serde_json::Value::Bool(false);
        }
    }
    set_json_setting(
        conn,
        EXAM_ANALYSIS_TASKS_KEY,
        &serde_json::Value::Array(items),
    )
}

fn normalize_exam_causal_provider_result(
    provider_result: Result<String, String>,
) -> serde_json::Value {
    match provider_result {
        Ok(content) => {
            let cleaned = extract_json_payload(&strip_code_fences(&content));
            let parsed = serde_json::from_str::<serde_json::Value>(&cleaned).or_else(|_| {
                serde_json::from_str::<serde_json::Value>(&repair_unescaped_json_string_quotes(
                    &cleaned,
                ))
            });
            match parsed {
                Ok(mut value) => {
                    value["eligible"] = serde_json::Value::Bool(true);
                    value["providerStatus"] = serde_json::Value::String("completed".to_string());
                    value
                }
                Err(error) => serde_json::json!({
                    "eligible": true,
                    "providerStatus": "provider_failure",
                    "candidates": [],
                    "limitations": [format!("AI response schema was invalid: {error}")]
                }),
            }
        }
        Err(error) => serde_json::json!({
            "eligible": true,
            "providerStatus": "provider_failure",
            "candidates": [],
            "limitations": [format!("AI provider unavailable: {error}")]
        }),
    }
}

fn repair_unescaped_json_string_quotes(content: &str) -> String {
    let characters = content.chars().collect::<Vec<_>>();
    let mut repaired = String::with_capacity(content.len());
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in characters.iter().copied().enumerate() {
        if escaped {
            repaired.push(character);
            escaped = false;
            continue;
        }
        if in_string && character == '\\' {
            repaired.push(character);
            escaped = true;
            continue;
        }
        if character != '"' {
            repaired.push(character);
            continue;
        }
        if !in_string {
            in_string = true;
            repaired.push(character);
            continue;
        }

        let next_non_whitespace = characters[index + 1..]
            .iter()
            .copied()
            .find(|next| !next.is_whitespace());
        let closes_json_string = matches!(
            next_non_whitespace,
            Some(':') | Some(',') | Some('}') | Some(']') | None
        );
        if closes_json_string {
            in_string = false;
            repaired.push(character);
        } else {
            repaired.push('\\');
            repaired.push(character);
        }
    }

    repaired
}

fn normalize_exam_section_causal_provider_result(
    provider_result: Result<String, String>,
) -> serde_json::Value {
    let mut result = normalize_exam_causal_provider_result(provider_result);
    if result
        .get("questions")
        .and_then(serde_json::Value::as_array)
        .is_none()
    {
        result["questions"] = serde_json::json!([]);
    }
    result["candidates"] = serde_json::Value::Null;
    result
}

fn retain_marked_exam_section_candidates(
    result: &mut serde_json::Value,
    evidence: &[serde_json::Value],
    questions: &[PreparedExamSectionQuestion],
) {
    let expected: std::collections::HashSet<&str> = questions
        .iter()
        .map(|question| question.question_id.as_str())
        .collect();
    let Some(items) = result
        .get_mut("questions")
        .and_then(serde_json::Value::as_array_mut)
    else {
        result["questions"] = serde_json::json!([]);
        return;
    };
    items.retain_mut(|item| {
        item.get("questionId")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|question_id| expected.contains(question_id))
    });
    for question in questions {
        let item = items
            .iter_mut()
            .find(|item| item["questionId"] == question.question_id);
        if let Some(item) = item {
            item["questionNumber"] = serde_json::json!(question.question_number);
            item["stem"] = serde_json::json!(question.stem);
            item["selectedAnswer"] = serde_json::json!(question.selected_answer);
            item["correctAnswer"] = serde_json::json!(question.correct_answer);
            retain_marked_exam_causal_candidates(item, evidence);
        } else {
            items.push(serde_json::json!({
                "questionId": question.question_id,
                "questionNumber": question.question_number,
                "candidates": []
            }));
        }
    }
}

fn persist_exam_analysis(
    conn: &rusqlite::Connection,
    evidence_key: &str,
    paper_id: &str,
    question_id: &str,
    attempt_id: &str,
    status: &str,
    mut result: serde_json::Value,
) -> Result<String, String> {
    result["analysisVersion"] = serde_json::Value::String("exam-causal-v1".to_string());
    result["provider"] = serde_json::Value::String("configured".to_string());
    result["model"] = serde_json::Value::String("configured".to_string());
    let result_json = result.to_string();
    conn.execute(
        "INSERT INTO exam_question_analyses
            (evidence_key, paper_id, question_id, attempt_id, status, result_json, provider, model, analysis_version)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'configured', 'configured', 'exam-causal-v1')
         ON CONFLICT(evidence_key) DO UPDATE SET
            status = excluded.status,
            result_json = excluded.result_json,
            updated_at = datetime('now')",
        rusqlite::params![evidence_key, paper_id, question_id, attempt_id, status, result_json],
    )
    .map_err(|error| format!("Failed to persist exam analysis: {error}"))?;
    Ok(result_json)
}

pub fn save_exam_attempt(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let draft = word_storage_core::models::ExerciseAttemptDraft {
        attempt_id: required_string_field(&request, "attemptId")?,
        paper_id: required_string_field(&request, "paperId")?,
        section_id: required_string_field(&request, "sectionId")?,
        question_id: required_string_field(&request, "questionId")?,
        selected_answer: request
            .get("selectedAnswer")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        is_correct: request.get("isCorrect").and_then(|value| value.as_bool()),
        answer_history_json: serde_json::to_string(
            request
                .get("answerHistory")
                .unwrap_or(&serde_json::json!([])),
        )
        .map_err(|error| format!("Invalid answerHistory: {error}"))?,
        status: request
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("in_progress")
            .to_string(),
    };
    with_runtime_conn(|conn| {
        word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_attempt(conn, &draft)
            .map_err(|error| error.to_string())?;
        let attempt = word_storage_core::persistence::exercise_vocab_repo::get_exercise_attempt(
            conn,
            &draft.attempt_id,
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Saved exercise attempt was not found".to_string())?;
        serde_json::to_string(&attempt)
            .map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

pub fn get_exam_attempt(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let attempt_id = required_string_field(&request, "attemptId")?;
    with_runtime_conn(|conn| {
        let attempt = word_storage_core::persistence::exercise_vocab_repo::get_exercise_attempt(
            conn,
            &attempt_id,
        )
        .map_err(|error| error.to_string())?;
        let attempt = if let Some(attempt) = attempt {
            let article_id = format!("{}:{}", attempt.paper_id, attempt.section_id);
            let mut statement = conn
                .prepare(
                    "SELECT o.id, o.entry_id, o.word_form, o.normalized_form,
                            o.start_offset, o.end_offset, o.lookup_status, o.user_mark,
                            o.sentence_text, o.updated_at
                     FROM exercise_vocab_occurrences o
                     JOIN exercise_articles a ON a.id = o.article_id
                     WHERE a.article_id = ?1
                     ORDER BY o.start_offset",
                )
                .map_err(|error| format!("Failed to prepare attempt evidence query: {error}"))?;
            let evidence = statement
                .query_map([article_id], |row| {
                    Ok(serde_json::json!({
                        "occurrenceId": row.get::<_, i64>(0)?,
                        "entryId": row.get::<_, Option<i64>>(1)?,
                        "word": row.get::<_, String>(2)?,
                        "normalized": row.get::<_, String>(3)?,
                        "startOffset": row.get::<_, i64>(4)?,
                        "endOffset": row.get::<_, i64>(5)?,
                        "lookupStatus": row.get::<_, String>(6)?,
                        "userMark": row.get::<_, String>(7)?,
                        "sentenceText": row.get::<_, String>(8)?,
                        "updatedAt": row.get::<_, String>(9)?
                    }))
                })
                .map_err(|error| format!("Failed to load attempt vocabulary evidence: {error}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Failed to read attempt vocabulary evidence: {error}"))?;
            let mut value = serde_json::to_value(attempt)
                .map_err(|error| format!("Failed to serialize attempt: {error}"))?;
            value["vocabularyEvidence"] = serde_json::Value::Array(evidence);
            Some(value)
        } else {
            None
        };
        serde_json::to_string(&serde_json::json!({"attempt": attempt}))
            .map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

pub fn tokenize_exam_text(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let text = request
        .get("text")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    serde_json::to_string(&serde_json::json!({
        "offsetEncoding": "utf16",
        "tokens": exam_practice_domain::tokenize_english(text)
    }))
    .map_err(|error| format!("JSON serialization failed: {error}"))
}

fn public_exam_mark_level(mark: &str) -> &'static str {
    match mark {
        "fuzzy" | "uncertain" | "ignored" => "fuzzy",
        "familiar" => "familiar",
        "unknown" | "wrong" => "unknown",
        _ => "none",
    }
}

fn stored_exam_mark_level(user_mark: &str, mark_level: &str) -> &'static str {
    match mark_level {
        "fuzzy" => "fuzzy",
        "familiar" => "familiar",
        "unknown" => "unknown",
        _ => public_exam_mark_level(user_mark),
    }
}

pub fn inspect_exam_word(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let article_source_id = required_string_field(&request, "articleId")?;
    let word = required_string_field(&request, "word")?;
    let surface_normalized = word.to_ascii_lowercase();
    let start_offset = request
        .get("startOffset")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "Missing startOffset".to_string())?;
    let end_offset = request
        .get("endOffset")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "Missing endOffset".to_string())?;
    let requested_user_mark = request.get("userMark").and_then(|value| value.as_str());
    with_runtime(|runtime, conn| {
        let bundle_dir = runtime.paths().bundled_resource_path("");
        ensure_exam_dictionary_entries(conn, Some(&bundle_dir))?;
        let (entry_id, normalized) = resolve_exam_word_family(conn, &surface_normalized)?;
        let (_, mut meanings) = resolve_exam_dictionary_term(conn, &normalized, Some(&bundle_dir))?;
        if meanings.is_empty() {
            if let Some(meaning) =
                build_exam_phrase_fallback_meaning(conn, &normalized, Some(&bundle_dir))?
            {
                meanings.push(serde_json::Value::String(meaning));
            }
        }
        let article_id =
            word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_article(
                conn,
                &word_storage_core::models::ExerciseArticleDraft {
                    article_id: article_source_id,
                    source_type: request
                        .get("sourceType")
                        .and_then(|value| value.as_str())
                        .unwrap_or("builtin")
                        .to_string(),
                    title: request
                        .get("title")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    body: request
                        .get("body")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    language: "en".to_string(),
                    metadata_json: serde_json::to_string(
                        request.get("metadata").unwrap_or(&serde_json::json!({})),
                    )
                    .map_err(|error| format!("Invalid metadata: {error}"))?,
                },
            )
            .map_err(|error| error.to_string())?;
        let existing_mark = conn
            .query_row(
                "SELECT user_mark, mark_level
                 FROM exercise_vocab_occurrences
                 WHERE article_id = ?1 AND start_offset = ?2 AND end_offset = ?3
                   AND LOWER(word_form) = LOWER(?4)
                 LIMIT 1",
                rusqlite::params![article_id, start_offset, end_offset, word],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|error| format!("Failed to load existing exercise word mark: {error}"))?;
        let public_user_mark = requested_user_mark
            .map(public_exam_mark_level)
            .unwrap_or_else(|| {
                existing_mark
                    .as_ref()
                    .map(|(user_mark, mark_level)| stored_exam_mark_level(user_mark, mark_level))
                    .unwrap_or("none")
            });
        let persisted_user_mark = if public_user_mark == "none" {
            "none"
        } else {
            "unknown"
        };
        let persisted_mark_level = public_user_mark;
        let occurrence_id =
            word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_vocab_occurrence(
                conn,
                &word_storage_core::models::ExerciseVocabOccurrenceDraft {
                    article_id,
                    entry_id,
                    word_form: word.clone(),
                    normalized_form: normalized.clone(),
                    sentence_text: request
                        .get("sentenceText")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    paragraph_index: request
                        .get("paragraphIndex")
                        .and_then(|value| value.as_i64())
                        .unwrap_or(0),
                    sentence_index: request
                        .get("sentenceIndex")
                        .and_then(|value| value.as_i64())
                        .unwrap_or(0),
                    start_offset,
                    end_offset,
                    lookup_status: if entry_id.is_some() {
                        "matched"
                    } else {
                        "unmatched"
                    }
                    .to_string(),
                    user_mark: persisted_user_mark.to_string(),
                    meaning_note: meanings
                        .first()
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                },
            )
            .map_err(|error| error.to_string())?;
        if requested_user_mark.is_some() {
            if let Some(entry_id) = entry_id {
                word_storage_core::persistence::exercise_vocab_repo::mark_exercise_vocab_entry_in_article(
                    conn,
                    article_id,
                    entry_id,
                    persisted_user_mark,
                    persisted_mark_level,
                    meanings.first().and_then(|value| value.as_str()),
                )
                .map_err(|error| error.to_string())?;
            } else {
                word_storage_core::persistence::exercise_vocab_repo::mark_exercise_vocab_word_in_article(
                    conn,
                    article_id,
                    &normalized,
                    persisted_user_mark,
                    persisted_mark_level,
                    meanings.first().and_then(|value| value.as_str()),
                )
                .map_err(|error| error.to_string())?;
            }
        }
        word_storage_core::persistence::exercise_vocab_repo::refresh_same_article_relations(
            conn, article_id,
        )
        .map_err(|error| error.to_string())?;
        serde_json::to_string(&serde_json::json!({
            "occurrenceId": occurrence_id,
            "entryId": entry_id,
            "word": word,
            "normalized": normalized,
            "meanings": meanings,
            "userMark": public_user_mark,
            "isUnknown": matches!(public_user_mark, "fuzzy" | "familiar" | "unknown")
        }))
        .map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

pub fn get_exam_annotation_state(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let article_source_id = required_string_field(&request, "articleId")?;
    let surface_words = request
        .get("words")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| value.as_str().map(|word| word.to_ascii_lowercase()))
        .collect::<Vec<_>>();
    with_runtime(|runtime, conn| {
        let bundle_dir = runtime.paths().bundled_resource_path("");
        ensure_exam_dictionary_entries(conn, Some(&bundle_dir))?;
        let mut words = Vec::with_capacity(surface_words.len());
        for word in surface_words {
            let (_, normalized) = resolve_exam_word_family(conn, &word)?;
            if !words.contains(&normalized) {
                words.push(normalized);
            }
        }
        let marks =
            word_storage_core::persistence::exercise_vocab_repo::load_exercise_word_mark_state(
                conn,
                &article_source_id,
                &words,
            )
            .map_err(|error| error.to_string())?;
        let current_marks = marks
            .iter()
            .filter(|item| item.current_article)
            .map(|item| {
                let meaning = resolve_exam_mark_meaning(
                    conn,
                    &item.normalized_form,
                    &item.meaning,
                    Some(&bundle_dir),
                )?;
                Ok(serde_json::json!({
                    "normalized": item.normalized_form,
                    "meaning": meaning,
                    "mark": item.current_mark_level
                }))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let prior_marks = marks
            .iter()
            .filter(|item| item.prior_article)
            .map(|item| {
                let meaning = resolve_exam_mark_meaning(
                    conn,
                    &item.normalized_form,
                    &item.meaning,
                    Some(&bundle_dir),
                )?;
                Ok(serde_json::json!({
                    "normalized": item.normalized_form,
                    "meaning": meaning,
                    "mark": item.prior_mark_level
                }))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let annotations = match word_storage_core::persistence::exercise_vocab_repo::get_exercise_article_by_source_id(conn, &article_source_id)
            .map_err(|error| error.to_string())?
        {
            Some(article) => word_storage_core::persistence::exercise_vocab_repo::list_exercise_annotations(conn, article.id)
                .map_err(|error| error.to_string())?,
            None => Vec::new(),
        };
        let mut causal_statement = conn
            .prepare(
                "SELECT DISTINCT LOWER(o.normalized_form)
                 FROM exercise_vocab_occurrences o
                 JOIN exercise_articles a ON a.id = o.article_id
                 WHERE a.article_id = ?1 AND o.user_mark = 'wrong'",
            )
            .map_err(|error| format!("Failed to prepare causal-word query: {error}"))?;
        let causal_words = causal_statement
            .query_map([&article_source_id], |row| row.get::<_, String>(0))
            .map_err(|error| format!("Failed to load causal words: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Failed to read causal words: {error}"))?;
        serde_json::to_string(&serde_json::json!({
            "currentMarks": current_marks,
            "priorMarks": prior_marks,
            "causalWords": causal_words,
            "annotations": annotations
        }))
        .map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

fn resolve_exam_mark_meaning(
    conn: &word_storage_core::Connection,
    normalized: &str,
    stored_meaning: &str,
    bundle_dir: Option<&Path>,
) -> Result<String, String> {
    if !stored_meaning.trim().is_empty() {
        return Ok(stored_meaning.trim().to_string());
    }
    let (_, meanings) = resolve_exam_dictionary_term(conn, normalized, bundle_dir)?;
    Ok(meanings
        .iter()
        .filter_map(serde_json::Value::as_str)
        .find(|meaning| !meaning.trim().is_empty())
        .map(str::trim)
        .unwrap_or("")
        .to_string())
}

fn resolve_exam_dictionary_term(
    conn: &word_storage_core::Connection,
    normalized: &str,
    bundle_dir: Option<&Path>,
) -> Result<(Option<i64>, Vec<serde_json::Value>), String> {
    let mut entry_id = find_entry_id_by_word(conn, normalized)?;
    let alias_meanings = load_entry_alias_meaning_strings(conn, normalized)?;
    let meanings = if alias_meanings.is_empty() {
        match entry_id {
            Some(id) => load_entry_meaning_strings(conn, id)?,
            None => Vec::new(),
        }
    } else {
        alias_meanings
    };
    if !meanings.is_empty() {
        return Ok((entry_id, meanings));
    }
    let Some((commit, entry)) = bundle_dir
        .map(|dir| lookup_exam_corpus_dictionary_entry(dir, normalized))
        .transpose()?
        .flatten()
    else {
        return Ok((entry_id, meanings));
    };
    let imported_id = upsert_exam_corpus_dictionary_entry(conn, &commit, &entry)?;
    entry_id = Some(imported_id);
    Ok((
        entry_id,
        vec![serde_json::Value::String(entry.meaning.trim().to_string())],
    ))
}

fn build_exam_phrase_fallback_meaning(
    conn: &word_storage_core::Connection,
    phrase: &str,
    bundle_dir: Option<&Path>,
) -> Result<Option<String>, String> {
    let tokens = exam_practice_domain::tokenize_english(phrase);
    if tokens.len() < 2 || tokens.len() > 8 {
        return Ok(None);
    }
    let mut parts = Vec::with_capacity(tokens.len());
    for token in tokens {
        let (_, meanings) = resolve_exam_dictionary_term(conn, &token.normalized, bundle_dir)?;
        let Some(meaning) = meanings
            .iter()
            .filter_map(|value| value.as_str())
            .find(|value| !value.trim().is_empty())
            .map(str::trim)
        else {
            return Ok(None);
        };
        parts.push(format!("{}（{}）", token.text, meaning));
    }
    Ok(Some(format!("未收录固定词组；逐词：{}", parts.join(" / "))))
}

pub fn save_exam_annotation(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|error| format!("Invalid request: {error}"))?;
    let article_source_id = required_string_field(&request, "articleId")?;
    let annotation_id = required_string_field(&request, "annotationId")?;
    let selected_text = required_string_field(&request, "selectedText")?;
    let start_offset = request
        .get("startOffset")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "Missing startOffset".to_string())?;
    let end_offset = request
        .get("endOffset")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "Missing endOffset".to_string())?;
    if end_offset <= start_offset {
        return Err("Annotation endOffset must be greater than startOffset".to_string());
    }
    with_runtime_conn(|conn| {
        let article_id =
            word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_article(
                conn,
                &word_storage_core::models::ExerciseArticleDraft {
                    article_id: article_source_id,
                    source_type: request
                        .get("sourceType")
                        .and_then(|value| value.as_str())
                        .unwrap_or("builtin")
                        .to_string(),
                    title: request
                        .get("title")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    body: request
                        .get("body")
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                        .to_string(),
                    language: "en".to_string(),
                    metadata_json: serde_json::to_string(
                        request.get("metadata").unwrap_or(&serde_json::json!({})),
                    )
                    .map_err(|error| format!("Invalid metadata: {error}"))?,
                },
            )
            .map_err(|error| error.to_string())?;
        word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_annotation(
            conn,
            &word_storage_core::models::ExerciseAnnotationDraft {
                annotation_id,
                article_id,
                question_id: request
                    .get("questionId")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                scope: request
                    .get("scope")
                    .and_then(|value| value.as_str())
                    .unwrap_or("passage")
                    .to_string(),
                start_offset,
                end_offset,
                selected_text,
                note_text: request
                    .get("noteText")
                    .and_then(|value| value.as_str())
                    .unwrap_or("")
                    .to_string(),
                color: "yellow".to_string(),
            },
        )
        .map_err(|error| error.to_string())?;
        let annotations =
            word_storage_core::persistence::exercise_vocab_repo::list_exercise_annotations(
                conn, article_id,
            )
            .map_err(|error| error.to_string())?;
        serde_json::to_string(&serde_json::json!({"annotations": annotations}))
            .map_err(|error| format!("JSON serialization failed: {error}"))
    })
}

fn required_string_field(value: &serde_json::Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(|item| item.as_str())
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("Missing {field}"))
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
        let bundle_dir = runtime.paths().bundled_resource_path("");
        enrich_root_affix_entries_from_assets(&mut entries, &bundle_dir)?;
        enrich_word_graph_relations_from_assets(&mut entries, &bundle_dir)?;
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

pub fn get_wrong_word_graph() -> Result<String, String> {
    with_runtime(|runtime, conn| {
        repair_seed_meaning_noise(conn)?;
        let mut entries = load_wrong_word_entries(conn)?;
        enrich_root_affix_entries_from_assets(
            &mut entries,
            &runtime.paths().bundled_resource_path(""),
        )?;
        let positions = get_json_setting(
            conn,
            "wrong_word_graph_positions_json",
            &serde_json::json!({}),
        )?;
        let today = today_date_string();
        let node_entry_ids = entries
            .iter()
            .filter_map(|entry| entry.get("entryId").and_then(|value| value.as_i64()))
            .filter(|entry_id| *entry_id > 0)
            .collect::<BTreeSet<_>>();
        let mut edges = load_wrong_word_graph_co_occurrence_edges(conn, &node_entry_ids)?;
        edges.extend(build_wrong_word_graph_precomputed_relation_edges(&entries));
        edges.extend(build_wrong_word_graph_root_family_edges(&entries));
        edges.extend(build_wrong_word_graph_similar_form_edges(&entries));
        edges.extend(build_wrong_word_graph_synonym_edges(&entries));
        let mut nodes = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            nodes.push(build_wrong_word_graph_node(
                entry, index, &positions, &today,
            ));
        }
        let (exercise_nodes, exercise_edges) = load_exercise_graph_evidence(conn, nodes.len())?;
        let mut node_ids = nodes
            .iter()
            .filter_map(|node| node.get("id").and_then(|value| value.as_str()))
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        for node in exercise_nodes {
            let Some(node_id) = node.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            if node_ids.insert(node_id.to_string()) {
                nodes.push(node);
            }
        }
        let mut edge_ids = edges
            .iter()
            .filter_map(|edge| edge.get("id").and_then(|value| value.as_str()))
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        for edge in exercise_edges {
            let Some(edge_id) = edge.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            if edge_ids.insert(edge_id.to_string()) {
                edges.push(edge);
            }
        }
        let payload = serde_json::json!({
            "version": 1,
            "generatedAt": chrono::Utc::now().to_rfc3339(),
            "coordinateSemantics": {
                "x": "semanticTopic",
                "y": "masteryLevel",
                "z": "reviewUrgency"
            },
            "nodes": nodes,
            "edges": edges,
            "relationLegend": [
                {"type": "similarForm", "label": "similar form", "color": "#FFFFFF"},
                {"type": "synonym", "label": "synonym", "color": "#53B87A"},
                {"type": "coOccurrence", "label": "co-occurrence", "color": "#D75B5B"},
                {"type": "rootFamily", "label": "root family", "color": "#8B5CC6"}
            ],
            "viewportHint": {
                "preferredOrientation": "landscape",
                "rightRail": true,
                "initialFocusEntryId": null
            }
        });
        clean_json_string(&payload)
    })
}

fn load_exercise_graph_evidence(
    conn: &rusqlite::Connection,
    node_offset: usize,
) -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>), String> {
    let mut stmt = conn
        .prepare(
            "SELECT o.id, o.entry_id, o.word_form, o.meaning_note,
                    a.article_id, a.title
             FROM exercise_vocab_occurrences o
             JOIN exercise_articles a ON a.id = o.article_id
             WHERE o.user_mark IN ('unknown', 'wrong')
             ORDER BY o.id ASC",
        )
        .map_err(|error| format!("Failed to prepare exercise graph occurrences: {error}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|error| format!("Failed to query exercise graph occurrences: {error}"))?;
    let mut occurrence_nodes = BTreeMap::<i64, String>::new();
    let mut nodes_by_id = BTreeMap::<String, serde_json::Value>::new();
    for (index, row) in rows.enumerate() {
        let (occurrence_id, entry_id, word, meaning, article_id, article_title) =
            row.map_err(|error| format!("Failed to read exercise graph occurrence: {error}"))?;
        let (node_id, graph_entry_id, entry_kind) = match entry_id.filter(|id| *id > 0) {
            Some(id) => (wrong_word_graph_position_key("word", id), id, "word"),
            None => (
                wrong_word_graph_position_key("exerciseWord", -occurrence_id),
                -occurrence_id,
                "exerciseWord",
            ),
        };
        occurrence_nodes.insert(occurrence_id, node_id.clone());
        nodes_by_id.entry(node_id.clone()).or_insert_with(|| {
            let angle = ((node_offset + index) as f64) * 0.83;
            serde_json::json!({
                "id": node_id,
                "entryId": graph_entry_id,
                "entryKind": entry_kind,
                "word": word,
                "primaryGloss": meaning,
                "meanings": if meaning.is_empty() { Vec::<String>::new() } else { vec![meaning.clone()] },
                "wrongCountToday": 0,
                "wrongCountTotal": 0,
                "lastWrongAt": null,
                "priorityScore": 5.0,
                "masteryScore": 0.0,
                "urgencyScore": 0.6,
                "position": {
                    "x": angle.cos() * 0.62,
                    "y": angle.sin() * 0.62,
                    "z": 0.6
                },
                "isUserPlaced": false,
                "positionUpdatedAt": null,
                "sources": [format!("{article_title} ({article_id})")]
            })
        });
    }
    drop(stmt);

    let mut relation_stmt = conn
        .prepare(
            "SELECT r.source_occurrence_id, r.target_occurrence_id,
                    a.article_id, a.title, r.relation_weight, a.metadata_json
             FROM exercise_vocab_relations r
             JOIN exercise_articles a ON a.id = r.article_id
             WHERE r.relation_type = 'same_article'
             ORDER BY r.id ASC",
        )
        .map_err(|error| format!("Failed to prepare exercise graph relations: {error}"))?;
    let relation_rows = relation_stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|error| format!("Failed to query exercise graph relations: {error}"))?;
    let mut edges = Vec::new();
    let mut seen = BTreeSet::new();
    for row in relation_rows {
        let (
            source_occurrence,
            target_occurrence,
            article_id,
            article_title,
            weight,
            metadata_json,
        ) = row.map_err(|error| format!("Failed to read exercise graph relation: {error}"))?;
        let metadata: serde_json::Value =
            serde_json::from_str(&metadata_json).unwrap_or_else(|_| serde_json::json!({}));
        let (Some(source_node), Some(target_node)) = (
            occurrence_nodes.get(&source_occurrence),
            occurrence_nodes.get(&target_occurrence),
        ) else {
            continue;
        };
        if source_node == target_node {
            continue;
        }
        let (left, right) = if source_node <= target_node {
            (source_node, target_node)
        } else {
            (target_node, source_node)
        };
        let edge_id = format!("coOccurrence:{left}:{right}:exerciseArticle:{article_id}");
        if !seen.insert(edge_id.clone()) {
            continue;
        }
        edges.push(serde_json::json!({
            "id": edge_id,
            "edgeId": edge_id,
            "sourceNodeId": left,
            "targetNodeId": right,
            "relation": "coOccurrence",
            "relationType": "coOccurrence",
            "weight": weight.clamp(0.0, 1.0),
            "sourceRefs": [format!("exerciseArticle:{article_id}")],
            "evidence": [{
                "type": "sameArticle",
                "articleId": article_id,
                "articleTitle": article_title,
                "paperId": metadata.get("paperId").cloned().unwrap_or(serde_json::Value::Null),
                "sectionId": metadata.get("sectionId").cloned().unwrap_or(serde_json::Value::Null),
                "questionId": metadata.get("questionId").cloned().unwrap_or(serde_json::Value::Null)
            }],
            "label": format!("同篇练习：{article_title}"),
            "isUserPinned": false
        }));
    }
    Ok((nodes_by_id.into_values().collect(), edges))
}

pub fn save_wrong_word_graph_position(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    let entry_id = request
        .get("entryId")
        .and_then(|value| value.as_i64())
        .ok_or_else(|| "Missing entryId".to_string())?;
    let entry_kind = request
        .get("entryKind")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("word");
    let position = request
        .get("position")
        .ok_or_else(|| "Missing position".to_string())?;
    let x = finite_f64_field(position, "x")?;
    let y = finite_f64_field(position, "y")?;
    let z = finite_f64_field(position, "z")?;
    let updated_at = chrono::Utc::now().to_rfc3339();
    let saved = serde_json::json!({
        "entryId": entry_id,
        "entryKind": entry_kind,
        "position": {"x": x, "y": y, "z": z},
        "isUserPlaced": true,
        "positionUpdatedAt": updated_at
    });

    with_runtime_conn(|conn| {
        let mut positions = get_json_setting(
            conn,
            "wrong_word_graph_positions_json",
            &serde_json::json!({}),
        )?;
        if !positions.is_object() {
            positions = serde_json::json!({});
        }
        if let Some(map) = positions.as_object_mut() {
            map.insert(
                wrong_word_graph_position_key(entry_kind, entry_id),
                saved.clone(),
            );
        }
        set_json_setting(conn, "wrong_word_graph_positions_json", &positions)?;
        clean_json_string(&saved)
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
        ensure_seed_vocabulary_available_for_today(
            conn,
            &runtime.paths().bundled_resource_path(""),
        )?;
        let today_state = build_authoritative_today_home_state(conn)?;
        let wrong_words = load_wrong_word_inputs_for_date(
            conn,
            &today_state.today_date,
            AI_PASSAGE_MAX_WRONG_WORDS,
        )?;

        let tasks_complete = today_state
            .today_snapshot
            .as_ref()
            .map(|snapshot| {
                snapshot.new_words_completed >= snapshot.new_words_target
                    && snapshot.review_words_completed >= snapshot.review_words_target
                    && snapshot.mixed_test_completed >= snapshot.mixed_test_target
                    && snapshot.wrong_word_test_completed >= snapshot.wrong_word_test_target
                    && snapshot.high_frequency_completed >= snapshot.high_frequency_target
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

pub fn get_ai_passage_style_preference() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let style = get_json_setting(
            conn,
            "ai_passage_style_preference_json",
            &serde_json::json!({"style": ""}),
        )?;
        clean_json_string(&style)
    })
}

pub fn save_ai_passage_style_preference(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    let style = request
        .get("style")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "AI passage style preference cannot be empty".to_string())?;
    if style.chars().count() > 240 {
        return Err("AI passage style preference is too long".to_string());
    }
    let payload = serde_json::json!({
        "style": style,
        "updatedAt": chrono::Utc::now().to_rfc3339(),
    });
    with_runtime_conn(|conn| {
        set_json_setting(conn, "ai_passage_style_preference_json", &payload)?;
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
        high_frequency_per_day: value
            .get("highFrequencyPerDay")
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
        question_type_weights_by_mode: Some(question_type_weights_by_mode_from_value(value)),
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
            "SELECT s.session_id, s.mode, r.question_id, r.answered_at
             FROM study_results r
             INNER JOIN study_sessions s ON s.session_id = r.session_id
             WHERE r.answered_at IS NOT NULL",
        )
        .map_err(|e| format!("Failed to prepare today completion query: {e}"))?;

    let mut new_words_completed = 0u32;
    let mut review_words_completed = 0u32;
    let mut mixed_test_completed = 0u32;
    let mut wrong_word_test_completed = 0u32;
    let mut high_frequency_completed = 0u32;
    let mut root_affix_completed = 0u32;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| format!("Failed to query today completions: {e}"))?;

    let mut seen_completed_questions = BTreeSet::new();
    for row in rows {
        let (session_id, mode, question_id, answered_at) =
            row.map_err(|e| format!("Failed to decode today completion row: {e}"))?;
        if active.session_ids.contains(&session_id) {
            continue;
        }
        if local_date_from_rfc3339(&answered_at).as_deref() != Some(today_date) {
            continue;
        }
        if !seen_completed_questions.insert((session_id, question_id)) {
            continue;
        }
        match normalize_stored_session_mode(&mode).as_str() {
            "newWord" => new_words_completed = new_words_completed.saturating_add(1),
            "review" => review_words_completed = review_words_completed.saturating_add(1),
            "mixedTest" => mixed_test_completed = mixed_test_completed.saturating_add(1),
            "wrongWordReinforcement" => {
                wrong_word_test_completed = wrong_word_test_completed.saturating_add(1)
            }
            "highFrequency" => {
                high_frequency_completed = high_frequency_completed.saturating_add(1)
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
    high_frequency_completed =
        high_frequency_completed.saturating_add(active.seed.high_frequency_completed);
    root_affix_completed =
        root_affix_completed.saturating_add(active.seed.root_affix_completed.unwrap_or(0));

    Ok(TodayCompletionSeed {
        new_words_completed,
        review_words_completed,
        mixed_test_completed,
        wrong_word_test_completed,
        high_frequency_completed,
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
        high_frequency_completed: 0,
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
        ("active_study_session_highFrequency", "highFrequency"),
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
        // The cursor is what the active study screen actually resumes from.
        // A failed partial save can leave stale result rows behind while the
        // cursor remains at the first unanswered question.
        let answered = snapshot
            .get("currentIndex")
            .and_then(|value| value.as_u64())
            .map(|value| value as u32)
            .or_else(|| snapshot_answered_question_count(&snapshot))
            .unwrap_or(0);

        match mode {
            "newWord" => seed.new_words_completed = answered,
            "review" => seed.review_words_completed = answered,
            "mixedTest" => seed.mixed_test_completed = answered,
            "wrongWordReinforcement" => seed.wrong_word_test_completed = answered,
            "highFrequency" => seed.high_frequency_completed = answered,
            "rootAffix" => seed.root_affix_completed = Some(answered),
            _ => {}
        }
    }

    Ok(ActiveSessionCompletionSeed { seed, session_ids })
}

fn snapshot_answered_question_count(snapshot: &serde_json::Value) -> Option<u32> {
    let results = snapshot.get("results").and_then(|value| value.as_array())?;
    let mut seen = BTreeSet::new();
    for result in results {
        if let Some(question_id) = result.get("questionId").and_then(|value| value.as_str()) {
            seen.insert(question_id.to_string());
        }
    }
    Some(seen.len() as u32)
}

fn snapshot_question_count(snapshot: &serde_json::Value) -> Option<u32> {
    snapshot
        .get("questions")
        .and_then(|value| value.as_array())
        .map(|items| items.len() as u32)
}

fn align_today_targets_to_active_sessions(
    _conn: &word_storage_core::Connection,
    targets: TodayTargetSeed,
    _today_date: &str,
) -> Result<TodayTargetSeed, String> {
    Ok(targets)
}

fn align_today_targets_to_completed_sessions(
    _conn: &word_storage_core::Connection,
    targets: TodayTargetSeed,
    _today_date: &str,
) -> Result<TodayTargetSeed, String> {
    Ok(targets)
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
        SessionMode::HighFrequency,
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

fn wrong_word_graph_position_key(entry_kind: &str, entry_id: i64) -> String {
    format!("{}:{}", entry_kind, entry_id)
}

fn finite_f64_field(value: &serde_json::Value, field: &str) -> Result<f64, String> {
    let number = value
        .get(field)
        .and_then(|item| item.as_f64())
        .ok_or_else(|| format!("Missing position.{field}"))?;
    if number.is_finite() {
        Ok(number)
    } else {
        Err(format!("position.{field} must be finite"))
    }
}

fn clamp01(value: f64) -> f64 {
    value.max(0.0).min(1.0)
}

fn wrong_word_graph_default_position(index: usize, error_count: i64) -> serde_json::Value {
    let angle = index as f64 * 2.399_963_229_728_653;
    let radius = 0.18 + (index as f64).sqrt() * 0.13;
    let urgency = clamp01(error_count.max(0) as f64 / 10.0);
    serde_json::json!({
        "x": (angle.cos() * radius * 1000.0).round() / 1000.0,
        "y": (angle.sin() * radius * 1000.0).round() / 1000.0,
        "z": (urgency * 1000.0).round() / 1000.0
    })
}

fn wrong_word_graph_primary_gloss(entry: &serde_json::Value) -> String {
    entry
        .get("meanings")
        .and_then(|value| value.as_array())
        .and_then(|items| items.first())
        .and_then(|item| {
            item.as_str().map(str::to_string).or_else(|| {
                item.get("meaning")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            })
        })
        .unwrap_or_default()
}

fn load_wrong_word_graph_co_occurrence_edges(
    conn: &rusqlite::Connection,
    allowed_entry_ids: &BTreeSet<i64>,
) -> Result<Vec<serde_json::Value>, String> {
    if allowed_entry_ids.len() < 2 {
        return Ok(Vec::new());
    }
    let mut history = get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
    clean_seed_meaning_noise_in_json(&mut history);
    let mut seen = BTreeSet::new();
    let mut edges = Vec::new();
    let passages = history.as_array().cloned().unwrap_or_default();
    for passage in passages {
        let passage_id = passage
            .get("passageId")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let passage_title = passage
            .get("title")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("AI 短文");
        let covered = passage
            .get("coveredWordIds")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        let mut ids = covered
            .iter()
            .filter_map(|value| value.as_i64())
            .filter(|entry_id| allowed_entry_ids.contains(entry_id))
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() < 2 {
            continue;
        }
        for source_index in 0..ids.len() {
            for target_index in (source_index + 1)..ids.len() {
                let source_id = ids[source_index];
                let target_id = ids[target_index];
                let edge_id = format!(
                    "coOccurrence:word:{source_id}:word:{target_id}:aiPassage:{passage_id}"
                );
                if !seen.insert(edge_id.clone()) {
                    continue;
                }
                edges.push(serde_json::json!({
                    "id": edge_id,
                    "edgeId": edge_id,
                    "sourceNodeId": wrong_word_graph_position_key("word", source_id),
                    "targetNodeId": wrong_word_graph_position_key("word", target_id),
                    "sourceEntryId": source_id,
                    "targetEntryId": target_id,
                    "relation": "coOccurrence",
                    "relationType": "coOccurrence",
                    "weight": 0.72,
                    "sourceRefs": ["aiPassage"],
                    "evidence": [{
                        "type": "aiPassage",
                        "passageId": passage_id,
                        "passageTitle": passage_title
                    }],
                    "label": format!("同篇 AI 短文：{passage_title}"),
                    "isUserPinned": false
                }));
                if edges.len() >= 120 {
                    return Ok(edges);
                }
            }
        }
    }
    Ok(edges)
}
fn build_wrong_word_graph_precomputed_relation_edges(
    entries: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let mut by_source_key = BTreeMap::<String, (&serde_json::Value, i64, String)>::new();
    for entry in entries {
        if entry
            .get("entryKind")
            .and_then(|value| value.as_str())
            .unwrap_or("word")
            != "word"
        {
            continue;
        }
        let Some(entry_id) = entry.get("entryId").and_then(|value| value.as_i64()) else {
            continue;
        };
        let Some(source_key) = entry.get("sourceEntryKey").and_then(|value| value.as_str()) else {
            continue;
        };
        by_source_key.insert(
            source_key.to_string(),
            (
                entry,
                entry_id,
                wrong_word_graph_position_key("word", entry_id),
            ),
        );
    }

    let mut edges = Vec::new();
    let mut seen = BTreeSet::new();
    for (source_key, (source_entry, source_entry_id, source_node_id)) in &by_source_key {
        let Some(relations) = source_entry.get("wordGraphRelations") else {
            continue;
        };
        append_precomputed_relation_edges(
            &mut edges,
            &mut seen,
            &by_source_key,
            source_key,
            *source_entry_id,
            source_node_id,
            relations.get("rootFamilyWords"),
            "rootFamily",
            "seedVocabRootFamily",
            "shared word family",
            0.74,
        );
        append_precomputed_relation_edges(
            &mut edges,
            &mut seen,
            &by_source_key,
            source_key,
            *source_entry_id,
            source_node_id,
            relations.get("similarFormWords"),
            "similarForm",
            "seedVocabSimilarForm",
            "similar form",
            0.62,
        );
        append_precomputed_relation_edges(
            &mut edges,
            &mut seen,
            &by_source_key,
            source_key,
            *source_entry_id,
            source_node_id,
            relations.get("meaningOverlapWords"),
            "synonym",
            "seedVocabMeaningOverlap",
            "shared meaning",
            0.66,
        );
        if edges.len() >= 240 {
            break;
        }
    }
    edges
}

fn append_precomputed_relation_edges(
    edges: &mut Vec<serde_json::Value>,
    seen: &mut BTreeSet<String>,
    by_source_key: &BTreeMap<String, (&serde_json::Value, i64, String)>,
    source_key: &str,
    source_entry_id: i64,
    source_node_id: &str,
    relation_items: Option<&serde_json::Value>,
    relation_type: &str,
    evidence_type: &str,
    label: &str,
    default_weight: f64,
) {
    let Some(items) = relation_items.and_then(|value| value.as_array()) else {
        return;
    };
    for item in items {
        let Some(target_key) = item.get("sourceId").and_then(|value| value.as_str()) else {
            continue;
        };
        if target_key == source_key {
            continue;
        }
        let Some((_target_entry, target_entry_id, target_node_id)) = by_source_key.get(target_key)
        else {
            continue;
        };
        let (left_node, right_node, left_entry, right_entry) =
            if source_node_id <= target_node_id.as_str() {
                (
                    source_node_id.to_string(),
                    target_node_id.clone(),
                    source_entry_id,
                    *target_entry_id,
                )
            } else {
                (
                    target_node_id.clone(),
                    source_node_id.to_string(),
                    *target_entry_id,
                    source_entry_id,
                )
            };
        let edge_key = format!("{relation_type}:{left_node}:{right_node}:{evidence_type}");
        if !seen.insert(edge_key.clone()) {
            continue;
        }
        let weight = item
            .get("weight")
            .and_then(|value| value.as_f64())
            .unwrap_or(default_weight);
        edges.push(serde_json::json!({
            "id": edge_key,
            "edgeId": edge_key,
            "sourceNodeId": left_node,
            "targetNodeId": right_node,
            "sourceEntryId": left_entry,
            "targetEntryId": right_entry,
            "relation": relation_type,
            "relationType": relation_type,
            "weight": clamp01(weight),
            "sourceRefs": [evidence_type],
            "evidence": [{
                "type": evidence_type,
                "targetSourceId": target_key,
                "targetWord": item.get("word").cloned().unwrap_or(serde_json::Value::Null),
                "sharedRoots": item.get("sharedRoots").cloned().unwrap_or(serde_json::Value::Null),
                "sharedMeaningTokens": item.get("sharedMeaningTokens").cloned().unwrap_or(serde_json::Value::Null),
                "distance": item.get("distance").cloned().unwrap_or(serde_json::Value::Null),
                "commonPrefix": item.get("commonPrefix").cloned().unwrap_or(serde_json::Value::Null)
            }],
            "label": label,
            "isUserPinned": false
        }));
    }
}
fn build_wrong_word_graph_root_family_edges(
    entries: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    const PREFIXES: &[&str] = &[
        "anti", "auto", "bio", "circum", "co", "com", "con", "contra", "de", "dis", "extra", "geo",
        "inter", "macro", "micro", "multi", "post", "pre", "pro", "semi", "sub", "super", "tele",
        "trans", "tri", "un",
    ];
    const SUFFIXES: &[&str] = &[
        "able", "ance", "ence", "ible", "ical", "isation", "ization", "ise", "ize", "less", "ment",
        "ness", "ous", "sion", "tion", "tive",
    ];

    let mut family_nodes: BTreeMap<String, Vec<(String, i64, String)>> = BTreeMap::new();
    for entry in entries {
        if entry
            .get("entryKind")
            .and_then(|value| value.as_str())
            .unwrap_or("word")
            != "word"
        {
            continue;
        }
        let Some(entry_id) = entry.get("entryId").and_then(|value| value.as_i64()) else {
            continue;
        };
        let Some(word) = entry.get("word").and_then(|value| value.as_str()) else {
            continue;
        };
        let normalized = normalize_graph_word_for_family(word);
        if normalized.len() < 5 {
            continue;
        }

        for prefix in PREFIXES {
            if normalized.starts_with(prefix) && normalized.len() >= prefix.len() + 3 {
                family_nodes
                    .entry(format!("prefix:{prefix}"))
                    .or_default()
                    .push((
                        wrong_word_graph_position_key("word", entry_id),
                        entry_id,
                        normalized.clone(),
                    ));
            }
        }
        for suffix in SUFFIXES {
            if normalized.ends_with(suffix) && normalized.len() >= suffix.len() + 3 {
                family_nodes
                    .entry(format!("suffix:{suffix}"))
                    .or_default()
                    .push((
                        wrong_word_graph_position_key("word", entry_id),
                        entry_id,
                        normalized.clone(),
                    ));
            }
        }
    }

    let mut edges = Vec::new();
    let mut seen = BTreeSet::new();
    for (family, mut nodes) in family_nodes {
        nodes.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)));
        nodes.dedup_by(|a, b| a.0 == b.0);
        if nodes.len() < 2 {
            continue;
        }
        for source_index in 0..nodes.len() {
            for target_index in (source_index + 1)..nodes.len() {
                let (source_node_id, source_entry_id, source_word) = &nodes[source_index];
                let (target_node_id, target_entry_id, target_word) = &nodes[target_index];
                let edge_key = format!("rootFamily:{source_node_id}:{target_node_id}:{family}");
                if !seen.insert(edge_key.clone()) {
                    continue;
                }
                edges.push(serde_json::json!({
                    "id": edge_key,
                    "edgeId": edge_key,
                    "sourceNodeId": source_node_id,
                    "targetNodeId": target_node_id,
                    "sourceEntryId": source_entry_id,
                    "targetEntryId": target_entry_id,
                    "relation": "rootFamily",
                    "relationType": "rootFamily",
                    "weight": 0.64,
                    "sourceRefs": ["wordFamilyHeuristic"],
                    "evidence": [{
                        "type": "wordFamilyHeuristic",
                        "family": family,
                        "sourceWord": source_word,
                        "targetWord": target_word
                    }],
                    "label": "shared word family",
                    "isUserPinned": false
                }));
                if edges.len() >= 120 {
                    return edges;
                }
            }
        }
    }
    edges
}

fn normalize_graph_word_for_family(word: &str) -> String {
    word.chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}
fn build_wrong_word_graph_similar_form_edges(
    entries: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let mut words = Vec::new();
    for entry in entries {
        if entry
            .get("entryKind")
            .and_then(|value| value.as_str())
            .unwrap_or("word")
            != "word"
        {
            continue;
        }
        let Some(entry_id) = entry.get("entryId").and_then(|value| value.as_i64()) else {
            continue;
        };
        let Some(word) = entry.get("word").and_then(|value| value.as_str()) else {
            continue;
        };
        let normalized = normalize_graph_word_for_family(word);
        if normalized.len() >= 4 {
            words.push((
                wrong_word_graph_position_key("word", entry_id),
                entry_id,
                normalized,
            ));
        }
    }

    let mut edges = Vec::new();
    let mut seen = BTreeSet::new();
    for source_index in 0..words.len() {
        for target_index in (source_index + 1)..words.len() {
            let (source_node_id, source_entry_id, source_word) = &words[source_index];
            let (target_node_id, target_entry_id, target_word) = &words[target_index];
            if !wrong_word_graph_words_are_similar(source_word, target_word) {
                continue;
            }
            let edge_key = format!("similarForm:{source_node_id}:{target_node_id}");
            if !seen.insert(edge_key.clone()) {
                continue;
            }
            let distance = bounded_levenshtein(source_word, target_word, 3).unwrap_or(3);
            let weight = clamp01(1.0 - distance as f64 / 4.0).max(0.38);
            edges.push(serde_json::json!({
                "id": edge_key,
                "edgeId": edge_key,
                "sourceNodeId": source_node_id,
                "targetNodeId": target_node_id,
                "sourceEntryId": source_entry_id,
                "targetEntryId": target_entry_id,
                "relation": "similarForm",
                "relationType": "similarForm",
                "weight": weight,
                "sourceRefs": ["spellingHeuristic"],
                "evidence": [{
                    "type": "spellingHeuristic",
                    "sourceWord": source_word,
                    "targetWord": target_word,
                    "distance": distance
                }],
                "label": "similar form",
                "isUserPinned": false
            }));
            if edges.len() >= 120 {
                return edges;
            }
        }
    }
    edges
}

fn build_wrong_word_graph_synonym_edges(entries: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut meaning_nodes: BTreeMap<String, Vec<(String, i64, String)>> = BTreeMap::new();
    for entry in entries {
        if entry
            .get("entryKind")
            .and_then(|value| value.as_str())
            .unwrap_or("word")
            != "word"
        {
            continue;
        }
        let Some(entry_id) = entry.get("entryId").and_then(|value| value.as_i64()) else {
            continue;
        };
        let Some(word) = entry.get("word").and_then(|value| value.as_str()) else {
            continue;
        };
        let meaning = normalize_graph_gloss_for_synonym(&wrong_word_graph_primary_gloss(entry));
        if meaning.chars().count() < 2 || meaning.chars().count() > 24 {
            continue;
        }
        meaning_nodes.entry(meaning).or_default().push((
            wrong_word_graph_position_key("word", entry_id),
            entry_id,
            word.to_string(),
        ));
    }

    let mut edges = Vec::new();
    let mut seen = BTreeSet::new();
    for (meaning, mut nodes) in meaning_nodes {
        nodes.sort_by(|a, b| a.2.cmp(&b.2).then(a.1.cmp(&b.1)));
        nodes.dedup_by(|a, b| a.0 == b.0);
        if nodes.len() < 2 {
            continue;
        }
        for source_index in 0..nodes.len() {
            for target_index in (source_index + 1)..nodes.len() {
                let (source_node_id, source_entry_id, source_word) = &nodes[source_index];
                let (target_node_id, target_entry_id, target_word) = &nodes[target_index];
                let edge_key = format!("synonym:{source_node_id}:{target_node_id}:{meaning}");
                if !seen.insert(edge_key.clone()) {
                    continue;
                }
                edges.push(serde_json::json!({
                    "id": edge_key,
                    "edgeId": edge_key,
                    "sourceNodeId": source_node_id,
                    "targetNodeId": target_node_id,
                    "sourceEntryId": source_entry_id,
                    "targetEntryId": target_entry_id,
                    "relation": "synonym",
                    "relationType": "synonym",
                    "weight": 0.58,
                    "sourceRefs": ["sharedPrimaryGloss"],
                    "evidence": [{
                        "type": "sharedPrimaryGloss",
                        "meaning": meaning,
                        "sourceWord": source_word,
                        "targetWord": target_word
                    }],
                    "label": "shared meaning",
                    "isUserPinned": false
                }));
                if edges.len() >= 120 {
                    return edges;
                }
            }
        }
    }
    edges
}

fn wrong_word_graph_words_are_similar(source: &str, target: &str) -> bool {
    if source == target {
        return false;
    }
    let length_delta = source.len().abs_diff(target.len());
    if length_delta > 3 {
        return false;
    }
    if source.chars().next() == target.chars().next() {
        if let Some(distance) = bounded_levenshtein(source, target, 2) {
            return distance > 0 && distance <= 2;
        }
    }
    common_ascii_prefix_len(source, target) >= 4
}

fn common_ascii_prefix_len(source: &str, target: &str) -> usize {
    source
        .bytes()
        .zip(target.bytes())
        .take_while(|(a, b)| a == b)
        .count()
}

fn bounded_levenshtein(source: &str, target: &str, max_distance: usize) -> Option<usize> {
    let source_len = source.chars().count();
    let target_len = target.chars().count();
    if source_len.abs_diff(target_len) > max_distance {
        return None;
    }
    let mut previous = (0..=target_len).collect::<Vec<_>>();
    let target_chars = target.chars().collect::<Vec<_>>();
    for (source_index, source_char) in source.chars().enumerate() {
        let mut current = Vec::with_capacity(target_len + 1);
        current.push(source_index + 1);
        let mut row_min = current[0];
        for (target_index, target_char) in target_chars.iter().enumerate() {
            let substitution = if source_char == *target_char { 0 } else { 1 };
            let value = (previous[target_index + 1] + 1)
                .min(current[target_index] + 1)
                .min(previous[target_index] + substitution);
            row_min = row_min.min(value);
            current.push(value);
        }
        if row_min > max_distance {
            return None;
        }
        previous = current;
    }
    previous
        .last()
        .copied()
        .filter(|distance| *distance <= max_distance)
}

fn normalize_graph_gloss_for_synonym(gloss: &str) -> String {
    gloss
        .trim()
        .trim_matches(|ch: char| ch.is_ascii_punctuation() || ch.is_whitespace())
        .to_lowercase()
}
fn build_wrong_word_graph_node(
    entry: &serde_json::Value,
    index: usize,
    positions: &serde_json::Value,
    today: &str,
) -> serde_json::Value {
    let entry_id = entry
        .get("entryId")
        .and_then(|value| value.as_i64())
        .unwrap_or_default();
    let entry_kind = entry
        .get("entryKind")
        .and_then(|value| value.as_str())
        .unwrap_or("word");
    let error_count = entry
        .get("errorCount")
        .and_then(|value| value.as_i64())
        .unwrap_or_default();
    let last_wrong_at = entry.get("lastWrongAt").and_then(|value| value.as_str());
    let today_error_count = if last_wrong_at
        .map(|value| value.starts_with(today))
        .unwrap_or(false)
    {
        error_count
    } else {
        0
    };
    let position_key = wrong_word_graph_position_key(entry_kind, entry_id);
    let saved = positions.get(&position_key);
    let position = saved
        .and_then(|value| value.get("position"))
        .cloned()
        .unwrap_or_else(|| wrong_word_graph_default_position(index, error_count));
    let is_user_placed = saved
        .and_then(|value| value.get("isUserPlaced"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let position_updated_at = saved
        .and_then(|value| value.get("positionUpdatedAt"))
        .and_then(|value| value.as_str());
    let urgency_score = clamp01(error_count.max(0) as f64 / 10.0);
    let mastery_score = clamp01(1.0 - urgency_score);
    let sources = if entry
        .get("isImported")
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        serde_json::json!(["wrongWord", "wrongWordImport"])
    } else {
        serde_json::json!(["wrongWord"])
    };

    serde_json::json!({
        "id": position_key,
        "entryId": entry_id,
        "entryKind": entry_kind,
        "sourceEntryKey": entry.get("sourceEntryKey").cloned().unwrap_or(serde_json::Value::Null),
        "word": entry.get("word").cloned().unwrap_or_else(|| serde_json::json!("")),
        "primaryGloss": wrong_word_graph_primary_gloss(entry),
        "meanings": entry.get("meanings").cloned().unwrap_or_else(|| serde_json::json!([])),
        "wrongCountToday": today_error_count,
        "wrongCountTotal": error_count,
        "lastWrongAt": last_wrong_at,
        "priorityScore": entry.get("priorityScore").cloned().unwrap_or_else(|| serde_json::json!(urgency_score * 10.0)),
        "masteryScore": mastery_score,
        "urgencyScore": urgency_score,
        "position": position,
        "isUserPlaced": is_user_placed,
        "positionUpdatedAt": position_updated_at,
        "sources": sources
    })
}
fn load_wrong_word_entries(conn: &rusqlite::Connection) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "WITH stats AS (
             SELECT
                e.id,
                e.source_entry_key,
                e.word,
                e.phonetic_us,
                e.phonetic_uk,
                e.part_of_speech,
                SUM(CASE WHEN sr.outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"') THEN 1 ELSE 0 END) as error_count,
                MAX(CASE WHEN sr.outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"') THEN sr.answered_at ELSE NULL END) as last_wrong_at,
                SUM(CASE WHEN sr.outcome IN ('correct', 'fuzzyCorrect', '\"correct\"', '\"fuzzyCorrect\"') THEN 1 ELSE 0 END) as correct_count,
                MAX(CASE WHEN sr.outcome IN ('correct', 'fuzzyCorrect', '\"correct\"', '\"fuzzyCorrect\"') THEN sr.answered_at ELSE NULL END) as last_correct_at,
                SUM(CASE WHEN sr.question_type IN ('glossToRootInput', 'rootToGlossInput', '\"glossToRootInput\"', '\"rootToGlossInput\"') THEN 1 ELSE 0 END) as root_affix_attempts
             FROM study_results sr
             JOIN entries e ON e.id = sr.entry_id
             WHERE sr.outcome IN ('incorrect', 'skipped', '\"incorrect\"', '\"skipped\"',
                                  'correct', 'fuzzyCorrect', '\"correct\"', '\"fuzzyCorrect\"')
               AND NOT EXISTS (
                 SELECT 1 FROM mastered_entries me
                 WHERE me.entry_id = e.id OR me.source_entry_id = e.source_entry_key
               )
             GROUP BY e.id, e.source_entry_key, e.word, e.phonetic_us, e.phonetic_uk, e.part_of_speech
             HAVING error_count > 0
            )
            SELECT
                stats.*,
                (
                    SELECT COUNT(*)
                    FROM study_results sr2
                    WHERE sr2.entry_id = stats.id
                      AND sr2.outcome IN ('correct', 'fuzzyCorrect', '\"correct\"', '\"fuzzyCorrect\"')
                      AND sr2.answered_at > COALESCE(stats.last_wrong_at, '')
                ) as correct_since_last_wrong
            FROM stats
            ORDER BY last_wrong_at DESC",
        )
        .map_err(|e| format!("Failed to prepare wrong-word list query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            let entry_id: i64 = row.get(0)?;
            let part_of_speech = row.get::<_, Option<String>>(5)?.unwrap_or_default();
            let root_affix_attempts = row.get::<_, i64>(10)?;
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
                    "correctCount": row.get::<_, i64>(8)?,
                    "lastCorrectAt": row.get::<_, Option<String>>(9)?,
                    "correctSinceLastWrong": row.get::<_, i64>(11)?,
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
             FROM imported_wrong_words iww
             WHERE NOT EXISTS (
               SELECT 1 FROM mastered_entries me
               WHERE (iww.entry_id IS NOT NULL AND me.entry_id = iww.entry_id)
                  OR me.source_entry_id = iww.word
             )
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
                "correctCount": 0,
                "lastCorrectAt": serde_json::Value::Null,
                "correctSinceLastWrong": 0,
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
    value["entryId"] = serde_json::json!(entry_id);
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
    value["errorCount"] = serde_json::json!(persistence::word_hint_repo::counted_error_count(
        conn, entry_id
    )
    .map_err(|error| error.to_string())?);
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
        enrich_study_question_exam_mark(conn, question)?;
        enrich_study_question_derivational_family(conn, question)?;
    }
    enrich_answered_questions_exam_marks(conn, payload, payload_is_high_frequency_mode(payload))?;
    Ok(())
}

fn enrich_submit_response_hints(
    conn: &rusqlite::Connection,
    payload: &mut serde_json::Value,
    include_synonyms: bool,
) -> Result<(), String> {
    if let Some(question) = payload
        .get_mut("currentQuestion")
        .filter(|value| value.is_object())
    {
        enrich_study_question_hints(conn, question)?;
        enrich_study_question_exam_mark(conn, question)?;
        enrich_study_question_derivational_family(conn, question)?;
    }
    enrich_answered_questions_exam_marks(conn, payload, include_synonyms)?;
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
    let Some(entry_id) = question_entry_id(conn, question) else {
        question["userHint"] = serde_json::Value::Null;
        question["hasHint"] = serde_json::json!(false);
        question["hintSuggestions"] = serde_json::json!([]);
        question["errorCount"] = serde_json::json!(0);
        return Ok(());
    };
    attach_word_hint_fields(conn, entry_id, question)
}

fn enrich_answered_questions_exam_marks(
    conn: &rusqlite::Connection,
    payload: &mut serde_json::Value,
    include_synonyms: bool,
) -> Result<(), String> {
    let Some(answered) = payload
        .get_mut("answeredQuestions")
        .and_then(|value| value.as_array_mut())
    else {
        return Ok(());
    };
    for item in answered {
        if let Some(question) = item.get_mut("question").filter(|value| value.is_object()) {
            enrich_study_question_error_count(conn, question)?;
            enrich_study_question_exam_mark(conn, question)?;
            enrich_study_question_derivational_family(conn, question)?;
            if include_synonyms {
                enrich_study_question_synonym_groups(conn, question)?;
            } else {
                question["synonymGroups"] = serde_json::json!([]);
            }
        }
    }
    Ok(())
}

fn payload_is_high_frequency_mode(payload: &serde_json::Value) -> bool {
    payload
        .get("session")
        .and_then(|session| session.get("mode"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|mode| mode == "highFrequency")
}

fn enrich_study_question_error_count(
    conn: &rusqlite::Connection,
    question: &mut serde_json::Value,
) -> Result<(), String> {
    let error_count = question_entry_id(conn, question)
        .map(|entry_id| persistence::word_hint_repo::counted_error_count(conn, entry_id))
        .transpose()
        .map_err(|error| error.to_string())?
        .unwrap_or(0);
    question["errorCount"] = serde_json::json!(error_count);
    Ok(())
}

fn enrich_study_question_exam_mark(
    conn: &rusqlite::Connection,
    question: &mut serde_json::Value,
) -> Result<(), String> {
    let Some(entry_id) = question_entry_id(conn, question) else {
        question["examMarked"] = serde_json::json!(false);
        question["examMarkLevel"] = serde_json::Value::Null;
        return Ok(());
    };
    let mark_level = load_exam_mark_level_for_entry(conn, entry_id)?;
    question["examMarked"] = serde_json::json!(mark_level.is_some());
    question["examMarkLevel"] = mark_level
        .map(serde_json::Value::String)
        .unwrap_or(serde_json::Value::Null);
    Ok(())
}

fn enrich_study_question_derivational_family(
    conn: &rusqlite::Connection,
    question: &mut serde_json::Value,
) -> Result<(), String> {
    let Some(entry_id) = question_entry_id(conn, question) else {
        question["derivationalFamily"] = serde_json::json!([]);
        return Ok(());
    };
    let question_word = question
        .get("word")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let mut statement = conn
        .prepare(
            "SELECT alias, meaning_cn
             FROM entry_aliases
             WHERE entry_id = ?1
               AND source_kind = 'related_word'
               AND TRIM(alias) <> ''
               AND TRIM(meaning_cn) <> ''
             GROUP BY LOWER(TRIM(alias)), TRIM(meaning_cn)
             ORDER BY LOWER(TRIM(alias)) ASC, TRIM(meaning_cn) ASC",
        )
        .map_err(|error| format!("Failed to prepare study derivational family query: {error}"))?;
    let rows = statement
        .query_map([entry_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("Failed to query study derivational family: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to read study derivational family: {error}"))?;
    let question_branch = study_derivational_branch(&question_word);
    let mut grouped =
        std::collections::BTreeMap::<String, (String, std::collections::BTreeSet<String>)>::new();
    for (alias, meaning) in rows {
        let alias = alias.trim();
        let meaning = meaning.trim();
        if alias.is_empty() || meaning.is_empty() {
            continue;
        }
        let alias_key = alias.to_ascii_lowercase();
        if !question_branch.is_empty() && study_derivational_branch(&alias_key) != question_branch {
            continue;
        }
        let item = grouped
            .entry(alias_key)
            .or_insert_with(|| (alias.to_string(), std::collections::BTreeSet::new()));
        item.1.insert(meaning.to_string());
    }
    let family = grouped
        .into_values()
        .take(16)
        .map(|(word, meanings)| {
            serde_json::json!({
                "word": word,
                "meaning": meanings.into_iter().collect::<Vec<_>>().join("；"),
            })
        })
        .collect();
    question["derivationalFamily"] = serde_json::Value::Array(family);
    Ok(())
}

fn enrich_study_question_synonym_groups(
    conn: &rusqlite::Connection,
    question: &mut serde_json::Value,
) -> Result<(), String> {
    let Some(entry_id) = question_entry_id(conn, question) else {
        question["synonymGroups"] = serde_json::json!([]);
        return Ok(());
    };
    let cache_key = study_synonym_cache_key(conn, entry_id)?;
    if let Some(groups) = STUDY_HIGH_FREQUENCY_SYNONYM_CACHE
        .lock()
        .map_err(|_| "Failed to lock high-frequency synonym cache".to_string())?
        .get(&cache_key)
        .cloned()
    {
        question["synonymGroups"] = serde_json::Value::Array(groups);
        return Ok(());
    }
    let explicit_groups = load_explicit_study_synonym_groups(conn, entry_id)?;
    let mut source_statement = conn
        .prepare(
            "SELECT pos, meaning_cn
             FROM entry_meanings
             WHERE entry_id = ?1
               AND TRIM(meaning_cn) <> ''
             ORDER BY sort_order ASC, id ASC",
        )
        .map_err(|error| format!("Failed to prepare high-frequency synonym source: {error}"))?;
    let source_meanings = source_statement
        .query_map([entry_id], |row| {
            Ok((
                row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                row.get::<_, String>(1)?,
            ))
        })
        .map_err(|error| format!("Failed to query high-frequency synonym source: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to read high-frequency synonym source: {error}"))?;
    let mut source_glosses = Vec::<StudySynonymGloss>::new();
    for (pos, meaning) in source_meanings {
        for gloss in study_synonym_meaning_keys(&meaning) {
            if let Some(existing) = source_glosses.iter_mut().find(|item| item.meaning == gloss) {
                existing
                    .pos_categories
                    .extend(study_synonym_pos_categories(&pos));
            } else {
                source_glosses.push(StudySynonymGloss {
                    meaning: gloss,
                    pos_categories: study_synonym_pos_categories(&pos),
                });
            }
        }
    }
    if source_glosses.is_empty() {
        STUDY_HIGH_FREQUENCY_SYNONYM_CACHE
            .lock()
            .map_err(|_| "Failed to lock high-frequency synonym cache".to_string())?
            .insert(cache_key, explicit_groups.clone());
        question["synonymGroups"] = serde_json::Value::Array(explicit_groups);
        return Ok(());
    }
    let synonym_condition = (0..source_glosses.len())
        .map(|index| {
            format!(
                "INSTR(
                    '；' || REPLACE(REPLACE(REPLACE(REPLACE(TRIM(em.meaning_cn), ';', '；'), '，', '；'), ',', '；'), '、', '；') || '；',
                    '；' || ?{} || '；'
                ) > 0",
                index + 2
            )
        })
        .collect::<Vec<_>>()
        .join(" OR ");

    let mut statement = conn
        .prepare(&format!(
            "SELECT e.id, e.word, em.pos, em.meaning_cn, e.exam_rank
             FROM entries e
             INNER JOIN wordbook_entries we ON we.entry_id = e.id
             INNER JOIN wordbooks wb ON wb.id = we.wordbook_id
             INNER JOIN entry_meanings em ON em.entry_id = e.id
             WHERE wb.code = 'kaoyan'
               AND e.exam_frequency > 0
               AND e.exam_rank IS NOT NULL
               AND e.id <> ?1
               AND TRIM(em.meaning_cn) <> ''
               AND ({synonym_condition})
             GROUP BY e.id, e.word, em.pos, em.meaning_cn, e.exam_rank
             ORDER BY e.exam_rank ASC, LOWER(e.word) ASC, e.id ASC",
        ))
        .map_err(|error| format!("Failed to prepare high-frequency synonym query: {error}"))?;
    let mut query_params = Vec::<&dyn rusqlite::ToSql>::with_capacity(source_glosses.len() + 1);
    query_params.push(&entry_id);
    query_params.extend(
        source_glosses
            .iter()
            .map(|gloss| &gloss.meaning as &dyn rusqlite::ToSql),
    );
    let rows = statement
        .query_map(query_params.as_slice(), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(|error| format!("Failed to query high-frequency synonyms: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to read high-frequency synonyms: {error}"))?;
    let mut groups = vec![BTreeMap::<i64, (String, i64)>::new(); source_glosses.len()];
    for (candidate_id, word, pos, meaning, rank) in rows {
        let candidate_pos = study_synonym_pos_categories(&pos);
        let candidate_glosses = study_synonym_meaning_keys(&meaning);
        let Some(group_index) = source_glosses.iter().position(|source| {
            candidate_glosses.contains(&source.meaning)
                && study_synonym_pos_matches(&source.pos_categories, &candidate_pos)
        }) else {
            continue;
        };
        groups[group_index]
            .entry(candidate_id)
            .or_insert_with(|| (word.trim().to_string(), rank));
    }
    let fallback_groups = source_glosses
        .iter()
        .zip(groups)
        .filter_map(|(source, words)| {
            let mut words = words
                .into_iter()
                .map(|(entry_id, (word, rank))| (entry_id, word, rank))
                .collect::<Vec<_>>();
            words.sort_by(|left, right| {
                left.2
                    .cmp(&right.2)
                    .then_with(|| {
                        left.1
                            .to_ascii_lowercase()
                            .cmp(&right.1.to_ascii_lowercase())
                    })
                    .then_with(|| left.0.cmp(&right.0))
            });
            let words = words
                .into_iter()
                .take(8)
                .map(|(_, word, _)| word)
                .collect::<Vec<_>>();
            (!words.is_empty()).then(|| {
                serde_json::json!({
                    "meaning": source.meaning,
                    "words": words,
                })
            })
        })
        .take(8)
        .collect::<Vec<_>>();
    let synonym_groups = merge_study_synonym_groups(explicit_groups, fallback_groups);
    STUDY_HIGH_FREQUENCY_SYNONYM_CACHE
        .lock()
        .map_err(|_| "Failed to lock high-frequency synonym cache".to_string())?
        .insert(cache_key, synonym_groups.clone());
    question["synonymGroups"] = serde_json::Value::Array(synonym_groups);
    Ok(())
}

fn merge_study_synonym_groups(
    mut explicit_groups: Vec<serde_json::Value>,
    fallback_groups: Vec<serde_json::Value>,
) -> Vec<serde_json::Value> {
    for fallback in fallback_groups {
        let Some(meaning) = fallback.get("meaning").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let fallback_words = fallback
            .get("words")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if fallback_words.is_empty() {
            continue;
        }
        if let Some(explicit) = explicit_groups
            .iter_mut()
            .find(|group| group.get("meaning").and_then(serde_json::Value::as_str) == Some(meaning))
        {
            let mut merged_words = explicit
                .get("words")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>();
            for word in fallback_words {
                if !merged_words
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&word))
                {
                    merged_words.push(word);
                }
            }
            explicit["words"] =
                serde_json::json!(merged_words.into_iter().take(8).collect::<Vec<_>>());
        } else {
            explicit_groups.push(serde_json::json!({
                "meaning": meaning,
                "words": fallback_words.into_iter().take(8).collect::<Vec<_>>(),
            }));
        }
    }
    explicit_groups.into_iter().take(8).collect()
}

fn load_explicit_study_synonym_groups(
    conn: &rusqlite::Connection,
    entry_id: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let mut statement = conn
        .prepare(
            "SELECT TRIM(ea.meaning_cn), e.id, e.word, e.exam_rank, em.meaning_cn
             FROM entry_aliases ea
             INNER JOIN entries e
                ON e.word = ea.alias COLLATE NOCASE OR e.lemma = ea.alias COLLATE NOCASE
             INNER JOIN wordbook_entries we ON we.entry_id = e.id
             INNER JOIN wordbooks wb ON wb.id = we.wordbook_id
             INNER JOIN entry_meanings em ON em.entry_id = e.id
             WHERE ea.entry_id = ?1
               AND ea.source_kind = 'synonym'
               AND TRIM(ea.meaning_cn) <> ''
               AND TRIM(em.meaning_cn) <> ''
               AND wb.code = 'kaoyan'
               AND e.id <> ?1
               AND e.exam_frequency > 0
               AND e.exam_rank IS NOT NULL
             GROUP BY TRIM(ea.meaning_cn), e.id, e.word, e.exam_rank, TRIM(em.meaning_cn), ea.id
             ORDER BY ea.id ASC, e.exam_rank ASC, LOWER(e.word) ASC, e.id ASC",
        )
        .map_err(|error| format!("Failed to prepare explicit high-frequency synonyms: {error}"))?;
    let rows = statement
        .query_map([entry_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|error| format!("Failed to query explicit high-frequency synonyms: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to read explicit high-frequency synonyms: {error}"))?;
    let mut groups = Vec::<(String, BTreeMap<i64, (String, i64)>)>::new();
    for (source_meaning, candidate_id, word, rank, candidate_meaning) in rows {
        let candidate_senses = study_synonym_meaning_keys(&candidate_meaning);
        for source_sense in study_synonym_meaning_keys(&source_meaning) {
            if !candidate_senses.contains(&source_sense) {
                continue;
            }
            let group = groups
                .iter_mut()
                .find(|(existing, _)| existing == &source_sense);
            if let Some((_, words)) = group {
                words
                    .entry(candidate_id)
                    .or_insert_with(|| (word.trim().to_string(), rank));
            } else {
                let mut words = BTreeMap::new();
                words.insert(candidate_id, (word.trim().to_string(), rank));
                groups.push((source_sense, words));
            }
        }
    }
    Ok(groups
        .into_iter()
        .filter_map(|(meaning, words)| {
            let mut words = words
                .into_iter()
                .map(|(id, (word, rank))| (id, word, rank))
                .collect::<Vec<_>>();
            words.sort_by(|left, right| {
                left.2
                    .cmp(&right.2)
                    .then_with(|| {
                        left.1
                            .to_ascii_lowercase()
                            .cmp(&right.1.to_ascii_lowercase())
                    })
                    .then_with(|| left.0.cmp(&right.0))
            });
            let words = words
                .into_iter()
                .take(8)
                .map(|(_, word, _)| word)
                .collect::<Vec<_>>();
            (!words.is_empty()).then(|| serde_json::json!({"meaning": meaning, "words": words}))
        })
        .take(8)
        .collect())
}

#[derive(Clone)]
struct StudySynonymGloss {
    meaning: String,
    pos_categories: BTreeSet<String>,
}

fn study_synonym_cache_key(
    conn: &rusqlite::Connection,
    entry_id: i64,
) -> Result<(String, i64), String> {
    let source_commit = conn
        .query_row(
            "SELECT sv.source_commit
             FROM entries e
             INNER JOIN source_versions sv ON sv.id = e.source_version_id
             WHERE e.id = ?1",
            [entry_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| format!("Failed to load high-frequency synonym cache key: {error}"))?
        .flatten()
        .unwrap_or_else(|| format!("entry-{entry_id}"));
    Ok((source_commit, entry_id))
}

fn study_synonym_pos_categories(pos: &str) -> BTreeSet<String> {
    pos.to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphabetic())
        .filter_map(|token| match token {
            "adj" | "adjective" => Some("adj"),
            "adv" | "adverb" => Some("adv"),
            "pron" | "pronoun" => Some("pron"),
            "prep" | "preposition" => Some("prep"),
            "conj" | "conjunction" => Some("conj"),
            "interj" | "interjection" => Some("interj"),
            "v" | "vi" | "vt" | "verb" => Some("v"),
            "n" | "noun" => Some("n"),
            _ => None,
        })
        .map(str::to_string)
        .collect()
}

fn study_synonym_pos_matches(source: &BTreeSet<String>, candidate: &BTreeSet<String>) -> bool {
    source.is_empty() || candidate.is_empty() || !source.is_disjoint(candidate)
}

fn study_synonym_meaning_keys(meaning: &str) -> Vec<String> {
    meaning
        .split(&['；', ';', '，', ',', '、'][..])
        .map(str::trim)
        .filter(|part| {
            let length = part.chars().count();
            length >= 2 && length <= 24 && part.chars().any(is_cjk_character)
        })
        .map(str::to_string)
        .collect::<Vec<_>>()
}

fn is_cjk_character(character: char) -> bool {
    matches!(character, '\u{4e00}'..='\u{9fff}')
}

fn study_derivational_branch(word: &str) -> &str {
    let word = word.trim();
    if word.starts_with("realiz") || word.starts_with("realis") {
        "realize"
    } else if matches!(
        word,
        "real" | "reality" | "really" | "realism" | "realist" | "realness"
    ) {
        "real"
    } else {
        ""
    }
}

fn load_exam_mark_level_for_entry(
    conn: &rusqlite::Connection,
    entry_id: i64,
) -> Result<Option<String>, String> {
    let row = conn
        .query_row(
            "SELECT CASE
                    WHEN o.user_mark = 'wrong' THEN 'wrong'
                    WHEN o.mark_level = 'unknown'
                        OR (o.mark_level = 'none' AND o.user_mark = 'unknown') THEN 'unknown'
                    WHEN o.mark_level = 'familiar' THEN 'familiar'
                    WHEN o.mark_level = 'fuzzy'
                        OR (o.mark_level = 'none' AND o.user_mark = 'ignored') THEN 'fuzzy'
                    ELSE ''
                END AS level
             FROM exercise_vocab_occurrences o
             WHERE o.entry_id = ?1
             ORDER BY CASE
                    WHEN o.user_mark = 'wrong' THEN 100
                    WHEN o.mark_level = 'unknown'
                        OR (o.mark_level = 'none' AND o.user_mark = 'unknown') THEN 80
                    WHEN o.mark_level = 'familiar' THEN 55
                    WHEN o.mark_level = 'fuzzy'
                        OR (o.mark_level = 'none' AND o.user_mark = 'ignored') THEN 35
                    ELSE 0
                END DESC,
                o.updated_at DESC
             LIMIT 1",
            [entry_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| format!("Failed to load exam mark level: {e}"))?;
    Ok(row.filter(|level| !level.trim().is_empty()))
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
    let Some(entry_id) =
        result_entry_id(conn, result).or_else(|| question_entry_id(conn, current_question))
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
            "label": "AI hint",
            "wordbookCodes": wordbook_codes,
            "text": if normalized_pos.is_empty() {
                format!("When you see {word}, recall its core meaning first, then compare nearby choices.")
            } else {
                format!("{word} is {normalized_pos}; use the part of speech to narrow the answer first.")
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

fn question_entry_id(conn: &rusqlite::Connection, question: &serde_json::Value) -> Option<i64> {
    question
        .get("entrySourceId")
        .and_then(|value| value.as_str())
        .and_then(|value| entry_id_from_source_value(conn, value))
}

fn result_entry_id(conn: &rusqlite::Connection, result: &serde_json::Value) -> Option<i64> {
    result
        .get("entrySourceId")
        .and_then(|value| value.as_str())
        .and_then(|value| entry_id_from_source_value(conn, value))
}

fn entry_id_from_source_value(conn: &rusqlite::Connection, value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(entry_id) = trimmed.parse::<i64>() {
        if entry_id > 0 {
            return Some(entry_id);
        }
    }
    conn.query_row(
        "SELECT id FROM entries WHERE source_entry_key = ?1 ORDER BY id ASC LIMIT 1",
        [trimmed],
        |row| row.get::<_, i64>(0),
    )
    .ok()
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

fn enrich_word_graph_relations_from_assets(
    entries: &mut [serde_json::Value],
    bundle_dir: &Path,
) -> Result<(), String> {
    let relations_by_key = seed_word_graph_relations_by_key(bundle_dir)?;
    if relations_by_key.is_empty() {
        return Ok(());
    }
    for entry in entries.iter_mut() {
        if entry
            .get("entryKind")
            .and_then(|value| value.as_str())
            .unwrap_or("word")
            != "word"
        {
            continue;
        }
        let Some(source_entry_key) = entry.get("sourceEntryKey").and_then(|value| value.as_str())
        else {
            continue;
        };
        if let Some(relations) = relations_by_key.get(source_entry_key) {
            entry["wordGraphRelations"] = relations.clone();
        }
    }
    Ok(())
}

fn seed_word_graph_relations_by_key(
    bundle_dir: &Path,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let book_dir = bundle_dir.join("seed-vocab").join("book");
    let mut out = BTreeMap::new();
    if !book_dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(&book_dir).map_err(|e| format!("Failed to read book assets: {e}"))? {
        let entry = entry.map_err(|e| format!("Failed to inspect book asset: {e}"))?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read book asset {}: {e}", path.display()))?;
        let items: Vec<serde_json::Value> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse book asset {}: {e}", path.display()))?;
        for item in items {
            let Some(relations) = item
                .get("wordGraphRelations")
                .or_else(|| item.pointer("/content/word/wordGraphRelations"))
                .or_else(|| item.pointer("/content/word/content/wordGraphRelations"))
                .cloned()
            else {
                continue;
            };
            let keys = [
                item.pointer("/content/word/wordId")
                    .and_then(|value| value.as_str()),
                item.get("displayWord").and_then(|value| value.as_str()),
                item.get("headWord").and_then(|value| value.as_str()),
                item.pointer("/content/word/wordHead")
                    .and_then(|value| value.as_str()),
            ];
            for key in keys.into_iter().flatten() {
                let trimmed = key.trim();
                if !trimmed.is_empty() {
                    out.entry(trimmed.to_string())
                        .or_insert_with(|| relations.clone());
                }
            }
        }
    }
    Ok(out)
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
    let entries = load_wrong_word_entries(conn)?;
    load_wrong_word_inputs_from_entries(conn, entries, limit)
}

fn load_wrong_word_inputs_for_date(
    conn: &rusqlite::Connection,
    today_date: &str,
    limit: usize,
) -> Result<Vec<serde_json::Value>, String> {
    let entries = load_wrong_word_entries(conn)?
        .into_iter()
        .filter(|entry| wrong_word_entry_matches_date(entry, today_date))
        .collect::<Vec<_>>();
    load_wrong_word_inputs_from_entries(conn, entries, limit)
}

fn wrong_word_entry_matches_date(entry: &serde_json::Value, today_date: &str) -> bool {
    entry
        .get("lastWrongAt")
        .and_then(|value| value.as_str())
        .and_then(local_date_from_rfc3339)
        .or_else(|| {
            entry
                .get("lastWrongAt")
                .and_then(|value| value.as_str())
                .and_then(|value| value.get(0..10).map(str::to_string))
        })
        .as_deref()
        == Some(today_date)
}

fn load_wrong_word_inputs_from_entries(
    conn: &rusqlite::Connection,
    mut entries: Vec<serde_json::Value>,
    limit: usize,
) -> Result<Vec<serde_json::Value>, String> {
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
            "partOfSpeech": part_of_speech,
            "todayWrongCount": entry.get("todayWrongCount").and_then(|value| value.as_i64()).unwrap_or(0),
            "errorCount": entry.get("errorCount").and_then(|value| value.as_i64()).unwrap_or(0)
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
    let mut values = stmt
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
    append_user_accepted_meaning_strings(conn, entry_id, &mut values)?;
    Ok(values)
}

fn load_entry_alias_meaning_strings(
    conn: &rusqlite::Connection,
    word: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT meaning_cn
             FROM entry_aliases
             WHERE alias = ?1 COLLATE NOCASE AND TRIM(meaning_cn) <> ''
             ORDER BY meaning_cn ASC",
        )
        .map_err(|error| format!("Failed to prepare entry alias meanings query: {error}"))?;
    let values = statement
        .query_map([word], |row| row.get::<_, String>(0))
        .map_err(|error| format!("Failed to query entry alias meanings: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to decode entry alias meaning: {error}"))?;
    Ok(values.into_iter().map(serde_json::Value::String).collect())
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
    let mut values = stmt
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
    append_user_accepted_meaning_details(conn, entry_id, &mut values)?;
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

pub fn get_reward_image_upload_entitlement() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let entitlement = load_reward_image_upload_entitlement(conn)?;
        Ok(entitlement.to_string())
    })
}

pub fn refresh_reward_image_upload_entitlement(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let current_streak_days = request
        .get("currentStreakDays")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0);
    let max_available_uploads = request
        .get("maxAvailableUploads")
        .and_then(|value| value.as_i64())
        .unwrap_or(3)
        .max(1);

    with_runtime_conn(|conn| {
        refresh_reward_image_entitlement_with_connection(
            conn,
            current_streak_days,
            max_available_uploads,
        )
        .map(|value| value.to_string())
    })
}

pub fn create_reward_image_upload(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let local_path = request
        .get("localPath")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("localPath is required")?;
    let mime_type = request
        .get("mimeType")
        .and_then(|value| value.as_str())
        .unwrap_or("image/jpeg")
        .trim();
    if !matches!(mime_type, "image/jpeg" | "image/png" | "image/webp") {
        return Err("mimeType must be image/jpeg, image/png, or image/webp".to_string());
    }
    let original_filename = request
        .get("originalFilename")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim();

    with_runtime_conn(|conn| {
        let current = load_reward_image_upload_entitlement(conn)?;
        let available_uploads = current
            .get("availableUploads")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        if available_uploads <= 0 {
            return Err("No reward image upload chances are available".to_string());
        }

        conn.execute(
            "UPDATE reward_image_upload_entitlements
             SET available_uploads = available_uploads - 1,
                 updated_at = datetime('now')
             WHERE owner_key = 'local' AND available_uploads > 0",
            [],
        )
        .map_err(|e| format!("Failed to consume upload entitlement: {e}"))?;

        let image = create_reward_image_upload_with_connection(
            conn,
            local_path,
            mime_type,
            original_filename,
        )?;
        Ok(image.to_string())
    })
}

pub fn list_reward_images(request_json: String) -> Result<String, String> {
    let request: serde_json::Value = if request_json.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?
    };
    let public_only = request
        .get("publicOnly")
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let week_start = request
        .get("weekStart")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(current_week_start_string);

    with_runtime_conn(|conn| {
        let images = list_reward_images_with_connection(conn, public_only, &week_start)?;
        Ok(images.to_string())
    })
}

pub fn moderate_reward_image(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let image_id = request
        .get("imageId")
        .and_then(|value| value.as_i64())
        .ok_or("imageId is required")?;
    let status = request
        .get("status")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .ok_or("status is required")?;
    let reason = request
        .get("reason")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim();

    with_runtime_conn(|conn| {
        let image = moderate_reward_image_with_connection(conn, image_id, status, reason)?;
        Ok(image.to_string())
    })
}

pub fn select_leaderboard_reward_image_tag(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let image_id = request
        .get("imageId")
        .and_then(|value| value.as_i64())
        .ok_or("imageId is required")?;

    with_runtime_conn(|conn| {
        let image = reward_image_by_id(conn, image_id)?;
        if image
            .get("moderationStatus")
            .and_then(|value| value.as_str())
            != Some("approved")
        {
            return Err("Only approved images can be selected as leaderboard tags".to_string());
        }
        conn.execute(
            "INSERT INTO leaderboard_image_tags (owner_key, image_id, updated_at)
             VALUES ('local', ?1, datetime('now'))
             ON CONFLICT(owner_key) DO UPDATE SET
                image_id = excluded.image_id,
                updated_at = excluded.updated_at",
            rusqlite::params![image_id],
        )
        .map_err(|e| format!("Failed to select leaderboard image tag: {e}"))?;
        Ok(image.to_string())
    })
}

pub fn vote_reward_image(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let image_id = request
        .get("imageId")
        .and_then(|value| value.as_i64())
        .ok_or("imageId is required")?;
    let week_start = request
        .get("weekStart")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(current_week_start_string);

    with_runtime_conn(|conn| {
        let result = vote_reward_image_with_connection(conn, image_id, "local", &week_start)?;
        Ok(result.to_string())
    })
}

pub fn refresh_local_leaderboard_summary(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let user_key = request
        .get("userKey")
        .and_then(|value| value.as_str())
        .unwrap_or("local")
        .trim();
    let display_name = request
        .get("displayName")
        .and_then(|value| value.as_str())
        .unwrap_or("Local learner")
        .trim();
    let period = request
        .get("period")
        .and_then(|value| value.as_str())
        .unwrap_or("all_time")
        .trim();
    let period_start = request
        .get("periodStart")
        .and_then(|value| value.as_str())
        .unwrap_or("1970-01-01")
        .trim();
    let total_questions = request
        .get("totalQuestions")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0);
    let correct_count = request
        .get("correctCount")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .clamp(0, total_questions);
    let mixed_test_total_questions = request
        .get("mixedTestTotalQuestions")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0);
    let mixed_test_correct_count = request
        .get("mixedTestCorrectCount")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .clamp(0, mixed_test_total_questions);
    let current_streak_days = request
        .get("currentStreakDays")
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0);

    with_runtime_conn(|conn| {
        let entry = upsert_local_leaderboard_summary_with_connection(
            conn,
            LocalLeaderboardSummaryInput {
                user_key,
                display_name,
                period,
                period_start,
                total_questions,
                correct_count,
                mixed_test_total_questions,
                mixed_test_correct_count,
                current_streak_days,
            },
        )?;
        Ok(entry.to_string())
    })
}

pub fn get_local_leaderboard(request_json: String) -> Result<String, String> {
    let request: serde_json::Value = if request_json.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?
    };
    let metric = request
        .get("metric")
        .and_then(|value| value.as_str())
        .unwrap_or("totalQuestions");
    let period = request
        .get("period")
        .and_then(|value| value.as_str())
        .unwrap_or("weekly");
    let period_start = request
        .get("periodStart")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let limit = request
        .get("limit")
        .and_then(|value| value.as_i64())
        .unwrap_or(50)
        .clamp(1, 100);

    with_runtime_conn(|conn| {
        let payload =
            get_local_leaderboard_with_connection(conn, metric, period, period_start, limit)?;
        Ok(payload.to_string())
    })
}

pub fn seed_local_leaderboard_demo() -> Result<String, String> {
    with_runtime(|runtime, conn| {
        let image_dir = runtime.paths().app_data_dir().join("reward_images");
        let payload = seed_local_leaderboard_demo_with_connection(conn, Some(&image_dir))?;
        Ok(payload.to_string())
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
        normalize_question_type_weights_on_plan(&mut plan);
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

pub fn get_croc_bti_profile() -> Result<String, String> {
    with_runtime_conn(|conn| {
        let profile = get_json_setting(conn, "croc_bti_profile_json", &serde_json::Value::Null)?;
        Ok(profile.to_string())
    })
}

pub fn save_croc_bti_profile(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;
    let profile = request
        .get("profile")
        .cloned()
        .ok_or("profile is required")?;
    let code = profile
        .get("resultCode")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("profile.resultCode is required")?;
    if code.len() != 4 {
        return Err("profile.resultCode must be a Croc BTI code".to_string());
    }

    with_runtime_conn(|conn| {
        set_json_setting(conn, "croc_bti_profile_json", &profile)?;
        let outbox_payload = serde_json::json!({
            "type": "croc_bti_profile_snapshot",
            "profile": profile,
        });
        enqueue_sync_snapshot(
            conn,
            "croc_bti_profile",
            &outbox_payload,
            "croc_bti_profile:current",
        );
        Ok(profile.to_string())
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

fn enqueue_shared_study_event(
    conn: &word_storage_core::Connection,
    question_id: &str,
    answered_at: &str,
) {
    match build_submitted_study_event_payload(conn, question_id, answered_at) {
        Ok(Some(payload)) => enqueue_sync_snapshot(
            conn,
            "study_events",
            &payload,
            "study_events:all_local_results",
        ),
        Ok(None) => {}
        Err(error) => {
            let payload = serde_json::json!({
                "type": "study_events_backfill",
                "error": error,
            });
            let _ = persistence::sync_repo::record_dead_letter(
                conn,
                "study_events",
                &payload.to_string(),
                "study_events:all_local_results",
                "snapshot_failed",
                payload
                    .get("error")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
            );
        }
    }
}

fn enqueue_study_sync_snapshots_after_submit(
    conn: &word_storage_core::Connection,
    submitted_result: Option<(&str, &str)>,
    is_complete: bool,
) {
    if let Some((question_id, answered_at)) = submitted_result {
        enqueue_shared_study_event(conn, question_id, answered_at);
    }
    if !is_complete {
        return;
    }
    enqueue_recent_study_word_points(conn);
    enqueue_wrong_word_entries_snapshot(conn);
}

fn should_enqueue_study_sync_snapshots_after_dispute_accept() -> bool {
    false
}

fn build_wrong_word_entries_payload(
    conn: &word_storage_core::Connection,
) -> Result<Option<serde_json::Value>, String> {
    cleanup_restored_wrong_word_projection_attempts(conn)?;
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

fn cleanup_restored_wrong_word_projection_attempts(
    conn: &word_storage_core::Connection,
) -> Result<usize, String> {
    conn.execute(
        "DELETE FROM study_results
         WHERE question_id LIKE 'cloud_restore:%:wrongWordReinforcement:%:enToCnChoice:%'
           AND outcome IN ('incorrect', '\"incorrect\"')
           AND response_time_ms = 0
           AND COALESCE(user_response, '') = ''
           AND COALESCE(correct_answer, '') = ''",
        [],
    )
    .map(|count| count as usize)
    .map_err(|e| format!("Failed to clean restored wrong-word projection attempts: {e}"))
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

fn build_study_events_payload(
    conn: &word_storage_core::Connection,
    result_id: Option<i64>,
) -> Result<Option<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT r.id, s.session_id, s.mode, r.question_id, r.entry_id,
                r.question_type, r.user_response, r.correct_answer, r.outcome,
                r.response_time_ms, r.answered_at, r.hint_used, e.source_entry_key,
                COALESCE(wb.code, (
                  SELECT linked.code FROM wordbook_entries linked_entry
                  JOIN wordbooks linked ON linked.id = linked_entry.wordbook_id
                  WHERE linked_entry.entry_id = r.entry_id
                  ORDER BY linked_entry.rank_in_book ASC, linked.id ASC LIMIT 1
                ), 'unassigned')
         FROM study_results r
         JOIN study_sessions s ON s.session_id = r.session_id
         JOIN entries e ON e.id = r.entry_id
         LEFT JOIN wordbooks wb ON wb.id = s.wordbook_id
         WHERE r.question_id NOT LIKE 'cloud_restore:%'
           AND r.question_id NOT LIKE 'cloud_event:%'
           AND s.session_id NOT LIKE 'cloud_restore:%'
           AND s.session_id NOT LIKE 'cloud_event:%'
           AND (?1 IS NULL OR r.id = ?1)
         ORDER BY r.id ASC",
        )
        .map_err(|e| format!("Failed to prepare shared study event query: {e}"))?;
    let rows = stmt
        .query_map([result_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, i64>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
            ))
        })
        .map_err(|e| format!("Failed to query shared study event rows: {e}"))?;
    let mut events = Vec::new();
    for row in rows {
        let (
            result_id,
            session_id,
            mode,
            question_id,
            entry_id,
            question_type,
            user_response,
            correct_answer,
            outcome,
            response_time_ms,
            answered_at,
            hint_used,
            entry_source_id,
            book_id,
        ) = row.map_err(|e| format!("Failed to decode shared study event row: {e}"))?;
        let mode = normalize_persisted_enum_text(&mode);
        let question_type = match normalize_persisted_enum_text(&question_type).as_str() {
            "meaning" | "choice" | "unknown" => "enToCnChoice".to_string(),
            "spelling" | "input" => "enToCnInput".to_string(),
            value => value.to_string(),
        };
        let outcome = normalize_persisted_enum_text(&outcome);
        if !matches!(
            mode.as_str(),
            "newWord"
                | "review"
                | "mixedTest"
                | "wrongWordReinforcement"
                | "highFrequency"
                | "rootAffix"
        ) || !matches!(
            question_type.as_str(),
            "enToCnChoice"
                | "exampleToCnChoice"
                | "exampleToCnChoiceNoTranslation"
                | "exampleComprehensionChoice"
                | "cnToEnChoice"
                | "enToCnInput"
                | "wordSkeletonInput"
                | "glossToRootInput"
                | "rootToGlossInput"
        ) || !matches!(
            outcome.as_str(),
            "correct" | "fuzzyCorrect" | "incorrect" | "skipped"
        ) || entry_id <= 0
            || entry_source_id.trim().is_empty()
            || answered_at.trim().is_empty()
        {
            continue;
        }
        let local_day = local_date_from_rfc3339(&answered_at)
            .unwrap_or_else(|| answered_at.get(0..10).unwrap_or(&answered_at).to_string());
        events.push(serde_json::json!({
            "localResultId": result_id,
            "sessionId": session_id,
            "occurredAt": answered_at,
            "payloadJson": {
                "schemaVersion": 1,
                "eventId": format!("word-mobile:{session_id}:{question_id}:{answered_at}"),
                "questionId": question_id,
                "entryId": entry_id,
                "entrySourceId": entry_source_id,
                "bookId": book_id,
                "bookVersion": "2026.1",
                "mode": mode,
                "questionType": question_type,
                "outcome": outcome,
                "userResponse": user_response,
                "canonicalAnswer": if correct_answer.trim().is_empty() { "(unavailable)" } else { correct_answer.as_str() },
                "responseTimeMs": response_time_ms.max(0),
                "hintUsed": hint_used != 0,
                "localDay": local_day,
                "legacyPointProjection": true
            }
        }));
    }
    if events.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::json!({
        "type": "study_events_backfill",
        "schemaVersion": 1,
        "events": events
    })))
}

fn build_all_study_events_payload(
    conn: &word_storage_core::Connection,
) -> Result<Option<serde_json::Value>, String> {
    build_study_events_payload(conn, None)
}

fn build_submitted_study_event_payload(
    conn: &word_storage_core::Connection,
    question_id: &str,
    answered_at: &str,
) -> Result<Option<serde_json::Value>, String> {
    let result_id = conn
        .query_row(
            "SELECT id FROM study_results WHERE question_id = ?1 AND answered_at = ?2 ORDER BY id DESC LIMIT 1",
            rusqlite::params![question_id, answered_at],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| format!("Failed to locate submitted study event: {e}"))?;
    match result_id {
        Some(result_id) => build_study_events_payload(conn, Some(result_id)),
        None => Ok(None),
    }
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
               AND r.question_id NOT LIKE 'cloud_restore:%'
               AND s.session_id NOT LIKE 'cloud_restore:%'
               AND s.session_id NOT LIKE 'cloud_event:%'
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
        let mut saved_plan = get_json_setting(conn, "saved_plan_json", &default_plan_json())?;
        normalize_question_type_weights_on_plan(&mut saved_plan);
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

    if text_content.trim().is_empty() {
        let payload = serde_json::json!({
            "batchId": format!("preview_{}", chrono::Utc::now().timestamp_millis()),
            "sourceType": source_type,
            "sourceName": source_name,
            "warnings": ["No text content was received for AI analysis."],
            "candidates": []
        });
        return serde_json::to_string(&payload)
            .map_err(|e| format!("JSON serialization failed: {}", e));
    }

    let raw_content = analyze_wrong_word_text_with_ai(source_name, mime_type, text_content)?;
    let parsed = parse_wrong_word_import_ai_output(&raw_content, source_type, source_name)?;
    serde_json::to_string(&parsed).map_err(|e| format!("JSON serialization failed: {}", e))
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

fn analyze_wrong_word_text_with_ai(
    source_name: &str,
    _mime_type: &str,
    text_content: &str,
) -> Result<String, String> {
    let system_message = WRONG_WORD_IMPORT_TEXT_PROMPT_TEMPLATE;
    let user_message = build_wrong_word_text_user_prompt(source_name, text_content)?;
    ai_agent().run_text_json(system_message, &user_message)
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
        "Extract vocabulary headwords from this image. If this is a wrong-word notebook or vocabulary app screenshot, scan the full image from top to bottom and extract the main English headword from EVERY visible vocabulary card/list item, not only the first card.\n\nFor a screenshot like a vertical wrong-word list, expected output should include all visible card titles such as cancel, defect, explosive, facilitate, and fridge when they are visible. Do not stop after one word.\n\nDo not extract bottom navigation labels, page titles, status bar text, button labels, dates, scores, phonetic spellings, or generic UI text. Do not infer words that are not visibly present, but you may correct obvious OCR misspellings when the visible word, phonetic line, and Chinese meaning all point to a standard vocabulary word, for example concel -> cancel.\n\n## Source Metadata\n```json\n{metadata}\n```\n\nReturn only the JSON object required by the system prompt."
    ))
}

fn build_wrong_word_text_user_prompt(
    source_name: &str,
    text_content: &str,
) -> Result<String, String> {
    let metadata = serde_json::to_string_pretty(&serde_json::json!({
        "sourceType": "text",
        "sourceName": source_name,
    }))
    .map_err(|e| format!("Failed to serialize import metadata: {e}"))?;
    let truncated = if text_content.len() > 8000 {
        &text_content[..8000]
    } else {
        text_content
    };
    Ok(format!(
        "Extract all English vocabulary words from the following text content. It may be pasted error logs, word lists, or study notes.\n\n## Source Metadata\n```json\n{metadata}\n```\n\n## Text Content\n```\n{truncated}\n```\n\nReturn only the JSON object required by the system prompt."
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
    let raw_word = object
        .get("word")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let word = normalize_import_ocr_headword(raw_word);
    let candidate_id = object
        .get("candidateId")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&word);
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
        "candidateId": normalize_import_ocr_headword(candidate_id),
        "word": word,
        "meaning": object.get("meaning").and_then(|value| value.as_str()).map(str::trim).filter(|value| !value.is_empty()),
        "occurrenceCount": occurrence_count,
        "confidence": confidence,
        "isDuplicate": object.get("isDuplicate").and_then(|value| value.as_bool()).unwrap_or(false),
        "isHighFrequency": object.get("isHighFrequency").and_then(|value| value.as_bool()).unwrap_or(occurrence_count >= 2),
        "evidence": evidence,
    }))
}

fn normalize_import_ocr_headword(word: &str) -> String {
    match word.trim().to_ascii_lowercase().as_str() {
        "concel" => "cancel".to_string(),
        value => value.to_string(),
    }
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

fn exam_word_family_candidates(word: &str) -> Vec<String> {
    let normalized = word.trim().to_ascii_lowercase();
    let mut candidates = Vec::new();
    let irregular = match normalized.as_str() {
        "children" => Some("child"),
        "people" => Some("person"),
        "men" => Some("man"),
        "women" => Some("woman"),
        "mice" => Some("mouse"),
        "feet" => Some("foot"),
        "teeth" => Some("tooth"),
        "geese" => Some("goose"),
        "went" | "gone" => Some("go"),
        "saw" | "seen" => Some("see"),
        "made" => Some("make"),
        "took" | "taken" => Some("take"),
        "gave" | "given" => Some("give"),
        "found" => Some("find"),
        "thought" => Some("think"),
        "bought" => Some("buy"),
        "brought" => Some("bring"),
        "wrote" | "written" => Some("write"),
        _ => None,
    };
    if let Some(irregular) = irregular {
        candidates.push(irregular.to_string());
    }
    if normalized.ends_with("ies") && normalized.len() > 3 {
        candidates.push(format!("{}y", &normalized[..normalized.len() - 3]));
    }
    if normalized.ends_with("ing") && normalized.len() > 4 {
        let stem = &normalized[..normalized.len() - 3];
        candidates.push(format!("{stem}e"));
        candidates.push(stem.to_string());
        if stem.len() > 2 {
            let bytes = stem.as_bytes();
            if bytes[stem.len() - 1] == bytes[stem.len() - 2] {
                candidates.push(stem[..stem.len() - 1].to_string());
            }
        }
    }
    if normalized.ends_with("ed") && normalized.len() > 3 {
        let stem = &normalized[..normalized.len() - 2];
        candidates.push(format!("{stem}e"));
        candidates.push(stem.to_string());
        if stem.len() > 2 {
            let bytes = stem.as_bytes();
            if bytes[stem.len() - 1] == bytes[stem.len() - 2] {
                candidates.push(stem[..stem.len() - 1].to_string());
            }
        }
    }
    if normalized.ends_with("es") && normalized.len() > 3 {
        candidates.push(normalized[..normalized.len() - 2].to_string());
    }
    const S_EXCEPTIONS: &[&str] = &["news", "series", "species", "means", "analysis"];
    if normalized.ends_with('s')
        && normalized.len() > 2
        && !normalized.ends_with("ss")
        && !S_EXCEPTIONS.contains(&normalized.as_str())
    {
        candidates.push(normalized[..normalized.len() - 1].to_string());
    }
    candidates.push(normalized);
    candidates.dedup();
    candidates
}

fn resolve_exam_word_family(
    conn: &rusqlite::Connection,
    word: &str,
) -> Result<(Option<i64>, String), String> {
    let surface = word.trim().to_ascii_lowercase();
    for candidate in exam_word_family_candidates(&surface) {
        let Some(entry_id) = find_entry_id_by_word(conn, &candidate)? else {
            continue;
        };
        let canonical = conn
            .query_row(
                "SELECT CASE WHEN TRIM(lemma) <> '' THEN LOWER(lemma) ELSE LOWER(word) END
                 FROM entries WHERE id = ?1",
                [entry_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(|error| format!("Failed to load canonical exam word: {error}"))?;
        return Ok((Some(entry_id), canonical));
    }
    Ok((None, surface))
}

fn find_entry_id_by_word(conn: &rusqlite::Connection, word: &str) -> Result<Option<i64>, String> {
    let normalized = word.trim().to_ascii_lowercase();
    let mut candidates = vec![normalized.clone()];
    if normalized.ends_with("ies") && normalized.len() > 3 {
        candidates.push(format!("{}y", &normalized[..normalized.len() - 3]));
    }
    if normalized.ends_with("es") && normalized.len() > 3 {
        candidates.push(normalized[..normalized.len() - 2].to_string());
    }
    if normalized.ends_with('s') && normalized.len() > 2 && !normalized.ends_with("ss") {
        candidates.push(normalized[..normalized.len() - 1].to_string());
    }
    if normalized.ends_with("ing") && normalized.len() > 4 {
        let stem = &normalized[..normalized.len() - 3];
        candidates.push(stem.to_string());
        candidates.push(format!("{stem}e"));
    }
    if normalized.ends_with("ed") && normalized.len() > 3 {
        let stem = &normalized[..normalized.len() - 2];
        candidates.push(stem.to_string());
        candidates.push(format!("{stem}e"));
    }
    if normalized.ends_with("ly") && normalized.len() > 3 {
        candidates.push(normalized[..normalized.len() - 2].to_string());
    }
    candidates.dedup();

    for candidate in candidates {
        let entry_id = conn
            .query_row(
                "SELECT id FROM entries
                 WHERE LOWER(word) = LOWER(?1) OR LOWER(lemma) = LOWER(?1)
                 ORDER BY CASE WHEN LOWER(word) = LOWER(?1) THEN 0 ELSE 1 END,
                          frequency DESC, id ASC
                 LIMIT 1",
                [&candidate],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| format!("Failed to match imported word to entry: {e}"))?;
        if entry_id.is_some() {
            return Ok(entry_id);
        }
        let alias_entry_id = conn
            .query_row(
                "SELECT ea.entry_id
                 FROM entry_aliases ea
                 JOIN entries e ON e.id = ea.entry_id
                 WHERE ea.alias = ?1 COLLATE NOCASE
                 ORDER BY e.frequency DESC, ea.entry_id ASC
                 LIMIT 1",
                [&candidate],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|error| format!("Failed to match entry alias: {error}"))?;
        if alias_entry_id.is_some() {
            return Ok(alias_entry_id);
        }
    }
    Ok(None)
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

    let all_wrong_words = resolve_ai_request_wrong_words(&request)?;
    if all_wrong_words.is_empty() {
        return Err("No wrong words available for passage generation".to_string());
    }
    let (wrong_words, other_wrong_words) = split_ai_passage_wrong_words(all_wrong_words);

    let request_style = request
        .get("style")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let stored_style = if request_style.is_none() {
        with_runtime_conn(|conn| {
            let preference = get_json_setting(
                conn,
                "ai_passage_style_preference_json",
                &serde_json::json!({"style": ""}),
            )?;
            Ok(preference
                .get("style")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string))
        })?
    } else {
        None
    };
    let style = request_style
        .map(str::to_string)
        .or(stored_style)
        .unwrap_or_else(|| "default".to_string());
    let system_message =
        format!("{PROMPT_TEMPLATE}\n\n## Active Style\n\n{DEFAULT_STYLE_TEMPLATE}");
    let user_message = build_ai_user_prompt(&wrong_words, &style, &date)?;

    let raw_content = ai_agent().run_ai_passage_json(&system_message, &user_message)?;
    let parsed = parse_ai_model_output(&raw_content, &wrong_words)?;
    let generated_at = chrono::Utc::now().to_rfc3339();
    let passage_id = format!("passage_{}", chrono::Utc::now().timestamp_millis());
    let mut blocks = parsed
        .get("blocks")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    append_other_wrong_words_block(&mut blocks, &other_wrong_words);
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
        "title": parsed.get("title").and_then(|value| value.as_str()).filter(|value| !value.is_empty()).unwrap_or("AI Passage"),
        "blocks": blocks,
        "wrongWords": wrong_words,
        "otherWrongWords": other_wrong_words,
        "coveredWordIds": covered_word_ids,
        "missingWordIds": missing_word_ids.clone(),
        "validationStatus": if missing_word_ids.is_empty() { "passed" } else { "failed" },
        "failureReason": if missing_word_ids.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::json!(format!("{} target words were not covered.", missing_word_ids.len()))
        },
        "wordCount": estimate_ai_word_count_from_blocks(parsed.get("blocks").and_then(|value| value.as_array()).unwrap_or(&Vec::new())),
        "targetLevel": level,
        "style": style,
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
        .map(|items| items.clone())
        .unwrap_or_default();
    if !wrong_words.is_empty() {
        return Ok(sort_ai_passage_wrong_words(wrong_words));
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
        let target_date = request.get("date").and_then(|value| value.as_str());
        let candidates = if let Some(date) = target_date {
            load_wrong_word_inputs_for_date(conn, date, AI_PASSAGE_MAX_WRONG_WORDS)?
        } else {
            load_wrong_word_inputs(conn, AI_PASSAGE_MAX_WRONG_WORDS)?
        };
        let mut candidates = sort_ai_passage_wrong_words(candidates);
        if !target_words.is_empty() {
            candidates.retain(|item| {
                item.get("word")
                    .and_then(|value| value.as_str())
                    .map(|word| target_words.contains(&word.trim().to_ascii_lowercase()))
                    .unwrap_or(false)
            });
        }
        Ok(candidates)
    })
}

fn split_ai_passage_wrong_words(
    wrong_words: Vec<serde_json::Value>,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let sorted = sort_ai_passage_wrong_words(wrong_words);
    let mut body_words = Vec::new();
    let mut other_words = Vec::new();
    for (index, item) in sorted.into_iter().enumerate() {
        if index < AI_PASSAGE_BODY_WORD_LIMIT {
            body_words.push(item);
        } else {
            other_words.push(item);
        }
    }
    (body_words, other_words)
}

fn sort_ai_passage_wrong_words(mut wrong_words: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    wrong_words.sort_by(|left, right| {
        json_i64_field(right, "todayWrongCount")
            .cmp(&json_i64_field(left, "todayWrongCount"))
            .then_with(|| {
                json_i64_field(right, "errorCount").cmp(&json_i64_field(left, "errorCount"))
            })
            .then_with(|| {
                json_str_field(left, "word")
                    .to_ascii_lowercase()
                    .cmp(&json_str_field(right, "word").to_ascii_lowercase())
            })
    });
    wrong_words
}

fn append_other_wrong_words_block(
    blocks: &mut Vec<serde_json::Value>,
    other_wrong_words: &[serde_json::Value],
) {
    if other_wrong_words.is_empty() {
        return;
    }
    let words = other_wrong_words
        .iter()
        .filter_map(|item| item.get("word").and_then(|value| value.as_str()))
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    if words.is_empty() {
        return;
    }
    blocks.push(serde_json::json!({
        "blockType": "paragraph",
        "text": format!("\u{5176}\u{4ed6}\u{9519}\u{8bcd}\u{ff1a}{}", words.join("\u{3001}"))
    }));
}

fn json_i64_field(value: &serde_json::Value, key: &str) -> i64 {
    value.get(key).and_then(|value| value.as_i64()).unwrap_or(0)
}

fn json_str_field<'a>(value: &'a serde_json::Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .unwrap_or("")
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
const DEFAULT_PRIMARY_AI_MODEL: &str = "claude-sonnet-4-20250514";
const DEFAULT_PRIMARY_AI_KEY: &str =
    "sk-181b95262e1eaebe50313d416f5ed81e38164b7d71494f3e241f3149e63a3e31";
const DEFAULT_ANTHROPIC_FALLBACK_AI_KEY: &str =
    "sk-c770a7ed5290da660e490ab9536b21e7af8ab9303bee61d417e3c0952170877f";
const DEFAULT_BACKUP_AI_URL: &str = "http://107.182.173.201:8080/v1/responses";
const DEFAULT_BACKUP_AI_MODEL: &str = "gpt-5.4";
const DEFAULT_AGNES_AI_URL: &str = "https://apihub.agnes-ai.com/v1";
const DEFAULT_AGNES_AI_MODEL: &str = "agnes-2.5-pro-alpha";
const DEFAULT_AGNES_AI_KEY: &str = "sk-VG8ZfaeFwPqqlHGguwxQuRxky4frZaGxEvTEJfY2xBMi9p9h";
const DEFAULT_BACKUP_AI_KEY: &str =
    "sk-d0fea41ec127dc71bbeb14da6a2507cc0bd7d92c740c512db67242d64358567d";
const AI_PASSAGE_BODY_WORD_LIMIT: usize = 10;
const AI_PASSAGE_MAX_WRONG_WORDS: usize = 100;
const PROMPT_TEMPLATE: &str = "You are a Chinese language learning assistant. Your task is to write a vivid, readable Chinese passage around specific English vocabulary words provided by the user.\n\nImportant: the backend will insert the actual English words and Chinese glosses. You should only decide where each word belongs inside the Chinese passage.\n\nYou will receive:\n- word_list: a JSON array of objects with word, primary_gloss, part_of_speech, entry_id\n- style: the desired writing style\n- length_target: approximate character count for the Chinese body text\n\nYou MUST call the write_ai_passage tool. Its arguments are the final passage payload with this structure:\n{\"title\":\"Optional contextual title\",\"paragraphs\":[\"Chinese paragraph with markers such as [[word:101]] inside the text.\"]}\n\nRules:\n1. Put the final passage only in the write_ai_passage tool arguments.\n2. Use [[word:ENTRY_ID]] exactly once per target word. Always use the literal label word, not the English headword.\n3. Do not output the English target words or glosses directly.\n4. Write natural Chinese paragraphs, not a word list.\n5. If there are many target words, write a longer passage with enough context for every word.\n6. Avoid default classroom or textbook scenes unless the words strongly require them.";
const DEFAULT_STYLE_TEMPLATE: &str = "Writing tone: imaginative, lively, and concrete while still easy to understand\nSentence length: Short to medium\nVocabulary level: Common Chinese vocabulary\nTopic connection: Use any fitting scene, such as travel, mystery, sci-fi, city life, dreams, myths, workplace drama, small adventures, or absurd comedy\nParagraph structure: adapt to the target word count\nCreativity: Prefer fresh situations over classroom explanations";
const WRONG_WORD_IMPORT_PROMPT_TEMPLATE: &str = "You are an English vocabulary extraction assistant for language learners. Analyze the provided image and extract only the vocabulary headwords that the learner is likely trying to import into a wrong-word notebook.\n\nTarget sources include screenshots of vocabulary apps, wrong-word notebooks, flashcards, printed word lists, handwritten word lists, or textbook pages with clear vocabulary entries.\n\nYou MUST return exactly one JSON object and nothing else:\n{\"sourceType\":\"image\",\"sourceName\":\"optional source name\",\"warnings\":[],\"candidates\":[{\"candidateId\":\"lowercase-word-or-stable-id\",\"word\":\"word\",\"meaning\":\"short Chinese meaning when visible next to that word, otherwise null\",\"occurrenceCount\":1,\"confidence\":0.0,\"isDuplicate\":false,\"isHighFrequency\":false,\"evidence\":\"brief evidence from the source\"}]}\n\nRules:\n1. For wrong-word notebook/app screenshots, scan from top to bottom and extract the main English headword of every visible vocabulary card/list item. Do not stop after the first recognized card. Examples: large card titles such as cancel, defect, explosive, facilitate, fridge.\n2. Correct obvious OCR mistakes in headwords only when nearby phonetics or Chinese glosses clearly identify the intended standard word, such as concel -> cancel.\n3. Ignore UI chrome and navigation text: page titles, tabs, buttons, bottom navigation labels, status bar text, badges, dates, scores, icons, labels such as AI/Today/Plan/Wrong/Reports, and any instructional copy.\n4. Ignore phonetic transcriptions and pronunciations. Do not output IPA-like text as a word.\n5. Do not infer or hallucinate. Return a candidate only when the English word is visibly present in the image.\n6. If a Chinese gloss is visibly adjacent to that headword on the same card/list item, include it in meaning. Otherwise set meaning to null.\n7. If image quality is low or a headword is partially obscured, include it only when still readable; lower confidence to 0.3-0.6 and explain the uncertainty in evidence.\n8. Set isHighFrequency true when the same headword appears 2+ times or is visually marked as high-priority. occurrenceCount must reflect actual visible count.\n9. evidence must describe where the headword appears, such as \"top of the second vocabulary card\" or \"left side of a word list row\".\n10. Only return an empty candidates array if no vocabulary headwords are visible.";

const WRONG_WORD_IMPORT_TEXT_PROMPT_TEMPLATE: &str = "You are an English vocabulary extraction assistant for language learners. Analyze the provided text content and extract every English word or phrase that could be a vocabulary item the learner is studying, has gotten wrong, or needs to review.\n\nTarget sources include: pasted error logs, word lists, study notes, exported data, CSV/JSON exports from vocabulary apps, or any text containing English vocabulary words alongside Chinese translations or study context.\n\nYou MUST return exactly one JSON object and nothing else:\n{\"sourceType\":\"text\",\"sourceName\":\"optional source name\",\"warnings\":[],\"candidates\":[{\"candidateId\":\"lowercase-word-or-stable-id\",\"word\":\"word\",\"meaning\":\"short Chinese meaning when present next to the word, otherwise null\",\"occurrenceCount\":1,\"confidence\":0.0,\"isDuplicate\":false,\"isHighFrequency\":false,\"evidence\":\"brief evidence from the source\"}]}\n\nRules:\n1. Extract English vocabulary words and phrases that a learner would need to study or review 闂?not generic English words like articles, prepositions, or common verbs unless they appear in a vocabulary-study context.\n2. Focus on words that appear alongside Chinese translations/glosses, error labels (闂傚倸鍊搁崐鎼佸磹閻戣姤鍊块柨鏃堟暜閸嬫挾绮☉妯诲闁稿绻濋弻鏇熺箾閻愵剚鐝曢梺? 闂傚倸鍊搁崐鎼佸磹閻戣姤鍊块柨鏃堟暜閸嬫挾绮☉妯诲闁稿绻濋弻鏇熺箾閻愵剚鐝﹂梺? 婵犵數濮烽弫鍛婃叏娴兼潙鍨傞柣鎾崇岸閺嬫牗绻涢幋鐐寸殤闁活厽鎸鹃埀顒冾潐濞叉牕煤閵堝棙鍙? etc.), difficulty markers, or other study-related annotations.\n3. If the text includes structured fields like word lists, CSV rows, or JSON, extract the vocabulary columns/fields.\n4. If Chinese translations or glosses appear next to English words, include them in the meaning field.\n5. Set isHighFrequency true when a word appears 2+ times or is marked as high-priority.\n6. occurrenceCount must be >= 1. confidence should be 0.0闂?.0 based on how clearly the word is identified as a vocabulary item (words with Chinese glosses: 0.85+; standalone words in study lists: 0.6闂?.8; generic words without context: 0.3闂?.5).\n7. evidence should describe WHERE the word was found and why it was selected (e.g. \"found in error log entry\", \"listed with Chinese gloss\", \"appears in vocabulary CSV\").\n8. Only return an empty candidates array if absolutely NO vocabulary-relevant English text is present.";

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
    #[serde(default)]
    anthropic_fallback: Option<StoredAiProviderProfile>,
    backup: StoredAiProviderProfile,
    #[serde(default)]
    agnes_fallback: Option<StoredAiProviderProfile>,
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
    anthropic_fallback: Option<AiProviderProfileSummary>,
    backup: AiProviderProfileSummary,
    agnes_fallback: Option<AiProviderProfileSummary>,
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

fn normalize_anthropic_model(model: &str) -> String {
    match model.trim() {
        "" | "claude" => DEFAULT_PRIMARY_AI_MODEL.to_string(),
        value => value.to_string(),
    }
}

fn default_ai_provider_config() -> StoredAiProviderConfig {
    StoredAiProviderConfig {
        primary: StoredAiProviderProfile {
            provider: "anthropic".to_string(),
            base_url: base_url_without_suffix(DEFAULT_PRIMARY_AI_URL, ANTHROPIC_MESSAGES_PATH),
            model: DEFAULT_PRIMARY_AI_MODEL.to_string(),
            auth_token: DEFAULT_PRIMARY_AI_KEY.to_string(),
        },
        anthropic_fallback: Some(StoredAiProviderProfile {
            provider: "anthropic".to_string(),
            base_url: base_url_without_suffix(DEFAULT_PRIMARY_AI_URL, ANTHROPIC_MESSAGES_PATH),
            model: DEFAULT_PRIMARY_AI_MODEL.to_string(),
            auth_token: DEFAULT_ANTHROPIC_FALLBACK_AI_KEY.to_string(),
        }),
        backup: StoredAiProviderProfile {
            provider: "openaiResponses".to_string(),
            base_url: base_url_without_suffix(DEFAULT_BACKUP_AI_URL, OPENAI_RESPONSES_PATH),
            model: DEFAULT_BACKUP_AI_MODEL.to_string(),
            auth_token: DEFAULT_BACKUP_AI_KEY.to_string(),
        },
        agnes_fallback: Some(StoredAiProviderProfile {
            provider: "openaiResponses".to_string(),
            base_url: DEFAULT_AGNES_AI_URL.to_string(),
            model: DEFAULT_AGNES_AI_MODEL.to_string(),
            auth_token: DEFAULT_AGNES_AI_KEY.to_string(),
        }),
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
        anthropic_fallback: config.anthropic_fallback.as_ref().map(to_summary),
        backup: to_summary(&config.backup),
        agnes_fallback: config.agnes_fallback.as_ref().map(to_summary),
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
    let default_config = default_ai_provider_config();
    let mut config = load_saved_ai_provider_config().unwrap_or_else(|| default_config.clone());
    if config.anthropic_fallback.is_none() {
        config.anthropic_fallback = default_config.anthropic_fallback.clone();
    }
    if config.agnes_fallback.is_none() {
        config.agnes_fallback = default_config.agnes_fallback.clone();
    }
    upgrade_legacy_agnes_fallback(config.agnes_fallback.as_mut());
    if let Some(base_url) = read_non_empty_env("ANTHROPIC_BASE_URL") {
        config.primary.base_url = base_url.trim_end_matches('/').to_string();
    }
    if let Some(model) = read_non_empty_env("ANTHROPIC_MODEL") {
        config.primary.model = normalize_anthropic_model(&model);
    }
    if let Some(token) = read_non_empty_env("ANTHROPIC_AUTH_TOKEN") {
        config.primary.auth_token = token;
    }
    config.primary.model = normalize_anthropic_model(&config.primary.model);
    if let Some(base_url) = read_non_empty_env("ANTHROPIC_FALLBACK_BASE_URL") {
        if let Some(profile) = config.anthropic_fallback.as_mut() {
            profile.base_url = base_url.trim_end_matches('/').to_string();
        }
    }
    if let Some(model) = read_non_empty_env("ANTHROPIC_FALLBACK_MODEL") {
        if let Some(profile) = config.anthropic_fallback.as_mut() {
            profile.model = normalize_anthropic_model(&model);
        }
    }
    if let Some(token) = read_non_empty_env("ANTHROPIC_FALLBACK_AUTH_TOKEN") {
        if let Some(profile) = config.anthropic_fallback.as_mut() {
            profile.auth_token = token;
        }
    }
    if let Some(profile) = config.anthropic_fallback.as_mut() {
        profile.model = normalize_anthropic_model(&profile.model);
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
    if let Some(base_url) = read_non_empty_env("AGNES_BASE_URL") {
        if let Some(profile) = config.agnes_fallback.as_mut() {
            profile.base_url = base_url.trim_end_matches('/').to_string();
        }
    }
    if let Some(model) = read_non_empty_env("AGNES_MODEL") {
        if let Some(profile) = config.agnes_fallback.as_mut() {
            profile.model = model;
        }
    }
    if let Some(token) = read_non_empty_env("AGNES_API_KEY") {
        if let Some(profile) = config.agnes_fallback.as_mut() {
            profile.auth_token = token;
        }
    }
    config
}

fn upgrade_legacy_agnes_fallback(profile: Option<&mut StoredAiProviderProfile>) {
    let Some(profile) = profile else {
        return;
    };
    let uses_official_endpoint = profile
        .base_url
        .trim_end_matches('/')
        .eq_ignore_ascii_case(DEFAULT_AGNES_AI_URL);
    let is_legacy_flash = matches!(
        profile.model.trim().to_ascii_lowercase().as_str(),
        "agnes-2.0-flash" | "agnes-2.5-flash"
    );
    if uses_official_endpoint && is_legacy_flash {
        profile.provider = "openaiResponses".to_string();
        profile.model = DEFAULT_AGNES_AI_MODEL.to_string();
    }
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
        anthropic_fallback: config.anthropic_fallback.map(|profile| AiProviderProfile {
            provider: profile.provider,
            base_url: profile.base_url,
            model: profile.model,
            auth_token: profile.auth_token,
        }),
        backup: AiProviderProfile {
            provider: config.backup.provider,
            base_url: config.backup.base_url,
            model: config.backup.model,
            auth_token: config.backup.auth_token,
        },
        agnes_fallback: config.agnes_fallback.map(|profile| AiProviderProfile {
            provider: profile.provider,
            base_url: profile.base_url,
            model: profile.model,
            auth_token: profile.auth_token,
        }),
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
    let marker = regex::Regex::new(r"\[\[(?:word|[A-Za-z][A-Za-z_-]*):(-?\d+)\]\]")
        .map_err(|e| e.to_string())?;
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
                    format!("{word} ({gloss})")
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
        "highFrequency",
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

fn open_study_session_action_database(
    runtime: &MobileRuntime,
) -> Result<word_storage_core::Connection, String> {
    let db_path = runtime.paths().database_path();
    if !db_path.exists() {
        return Err("Database not initialized".to_string());
    }
    if should_apply_schema_on_study_session_action() {
        return persistence::initialize_database(&db_path)
            .map_err(|e| format!("Failed to initialize database: {}", e));
    }
    persistence::open_database(&db_path).map_err(|e| format!("Failed to open database: {}", e))
}

fn should_apply_schema_on_study_session_action() -> bool {
    false
}

fn seed_vocabulary_has_entries(conn: &word_storage_core::Connection) -> Result<bool, String> {
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
    Ok(existing_entries > 0 && existing_links > 0)
}

fn ensure_seed_vocabulary_available_for_today(
    conn: &word_storage_core::Connection,
    bundle_dir: &Path,
) -> Result<(), String> {
    ensure_seed_vocabulary_imported(conn, bundle_dir)
}

fn ensure_seed_vocabulary_imported(
    conn: &word_storage_core::Connection,
    bundle_dir: &Path,
) -> Result<(), String> {
    const LEGACY_SEED_MAINTENANCE_KEY: &str = "seed_vocabulary_maintenance_v6_exam_frequency";
    const SEED_MAINTENANCE_KEY: &str = "seed_vocabulary_maintenance_v8_explicit_synonyms";
    let book_dir = bundle_dir.join("seed-vocab").join("book");
    if seed_vocabulary_has_entries(conn)? {
        let maintenance_done =
            get_json_setting(conn, SEED_MAINTENANCE_KEY, &serde_json::Value::Bool(false))?
                .as_bool()
                .unwrap_or(false);
        if maintenance_done {
            return Ok(());
        }
        if book_dir.exists() {
            refresh_seed_exam_frequency(conn, &book_dir.join("KaoYan_3.json"))?;
        }
        repair_seed_vocabulary_dedup(conn)?;
        if book_dir.exists() {
            refresh_seed_real_exam_examples(conn, &book_dir)?;
        }
        apply_seed_example_overrides(conn, bundle_dir)?;
        rebuild_seed_question_preps(conn)?;
        if book_dir.exists() {
            refresh_seed_precomputed_question_preps(conn, &book_dir)?;
            import_seed_aliases(conn, &book_dir)?;
        }
        set_json_setting(conn, SEED_MAINTENANCE_KEY, &serde_json::Value::Bool(true))?;
        set_json_setting(
            conn,
            LEGACY_SEED_MAINTENANCE_KEY,
            &serde_json::Value::Bool(true),
        )?;
        return Ok(());
    }

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
            .and_then(|_| repair_seed_vocabulary_dedup(conn))
            .and_then(|_| apply_seed_example_overrides(conn, bundle_dir))
            .and_then(|_| rebuild_seed_question_preps(conn))
            .and_then(|_| refresh_seed_precomputed_question_preps(conn, &book_dir))
            .and_then(|_| {
                set_json_setting(conn, SEED_MAINTENANCE_KEY, &serde_json::Value::Bool(true))
            })
            .and_then(|_| {
                set_json_setting(
                    conn,
                    LEGACY_SEED_MAINTENANCE_KEY,
                    &serde_json::Value::Bool(true),
                )
            }),
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

fn refresh_seed_exam_frequency(
    conn: &word_storage_core::Connection,
    path: &Path,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read exam-frequency vocabulary: {error}"))?;
    let words: Vec<serde_json::Value> = serde_json::from_str(&content)
        .map_err(|error| format!("Failed to parse exam-frequency vocabulary: {error}"))?;
    for item in words {
        let Some(word) = item
            .get("headWord")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let word_content = item.pointer("/content/word/content");
        let exam_frequency = word_content
            .and_then(|value| value.pointer("/realExamFrequency/occurrences"))
            .and_then(|value| value.as_i64())
            .unwrap_or(0)
            .max(0);
        let exam_rank = word_content
            .and_then(|value| value.pointer("/realExamFrequency/rank"))
            .and_then(|value| value.as_i64())
            .filter(|value| *value > 0);
        let part_of_speech = word_content
            .and_then(|value| value.get("trans"))
            .and_then(|value| value.as_array())
            .map(|translations| {
                let mut seen = std::collections::HashSet::new();
                translations
                    .iter()
                    .filter_map(|translation| {
                        translation.get("pos").and_then(|value| value.as_str())
                    })
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .filter(|value| seen.insert((*value).to_string()))
                    .collect::<Vec<_>>()
                    .join(" / ")
            })
            .unwrap_or_default();
        conn.execute(
            "UPDATE entries
             SET exam_frequency = ?1,
                 exam_rank = ?2,
                 part_of_speech = CASE WHEN ?3 <> '' THEN ?3 ELSE part_of_speech END
             WHERE id IN (
                 SELECT e.id
                 FROM entries e
                 INNER JOIN wordbook_entries we ON we.entry_id = e.id
                 INNER JOIN wordbooks wb ON wb.id = we.wordbook_id
                 WHERE wb.code = 'kaoyan' AND lower(e.word) = lower(?4)
             )",
            rusqlite::params![exam_frequency, exam_rank, part_of_speech, word],
        )
        .map_err(|error| format!("Failed to refresh exam frequency for {word}: {error}"))?;
    }
    Ok(())
}

fn refresh_seed_real_exam_examples(
    conn: &word_storage_core::Connection,
    book_dir: &Path,
) -> Result<(), String> {
    const BOOKS: [(&str, &str); 4] = [
        ("cet4", "CET4_3.json"),
        ("cet6", "CET6_3.json"),
        ("kaoyan", "KaoYan_3.json"),
        ("medical", "MEDICAL_RESP.json"),
    ];
    for (book_code, file_name) in BOOKS {
        let path = book_dir.join(file_name);
        if !path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|error| {
            format!("Failed to read real-exam examples from {file_name}: {error}")
        })?;
        let items: Vec<serde_json::Value> = serde_json::from_str(&raw).map_err(|error| {
            format!("Failed to parse real-exam examples from {file_name}: {error}")
        })?;
        let mut statement = conn
            .prepare(
                "SELECT e.id
                 FROM entries e
                 INNER JOIN wordbook_entries we ON we.entry_id = e.id
                 INNER JOIN wordbooks wb ON wb.id = we.wordbook_id
                 WHERE wb.code = ?1 AND lower(e.word) = lower(?2)",
            )
            .map_err(|error| format!("Failed to prepare real-exam entry lookup: {error}"))?;
        for item in items {
            let Some(word) = item
                .get("headWord")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|word| !word.is_empty())
            else {
                continue;
            };
            let word_content = item.pointer("/content/word/content");
            let entry_ids = statement
                .query_map(rusqlite::params![book_code, word], |row| {
                    row.get::<_, i64>(0)
                })
                .map_err(|error| format!("Failed to query real-exam entry {word}: {error}"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("Failed to read real-exam entry {word}: {error}"))?;
            for entry_id in entry_ids {
                conn.execute(
                    "DELETE FROM entry_examples
                     WHERE entry_id = ?1 AND sentence_cn LIKE ?2",
                    rusqlite::params![entry_id, "\u{771f}\u{9898}\u{6765}\u{6e90}\u{ff1a}%"],
                )
                .map_err(|error| {
                    format!("Failed to clear real-exam examples for {word}: {error}")
                })?;
                append_seed_entry_real_exam_examples(conn, entry_id, word_content)?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExamCorpusDictionaryAsset {
    source: ExamCorpusDictionarySource,
    entries: Vec<ExamCorpusDictionaryEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct ExamCorpusDictionarySource {
    commit: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExamCorpusDictionaryEntry {
    term: String,
    lemma: String,
    part_of_speech: String,
    meaning: String,
}

struct CachedExamCorpusDictionary {
    commit: String,
    entries: BTreeMap<String, ExamCorpusDictionaryEntry>,
}

fn ensure_exam_dictionary_entries(
    conn: &word_storage_core::Connection,
    _bundle_dir: Option<&Path>,
) -> Result<(), String> {
    ensure_curated_exam_dictionary_entries(conn)
}

fn lookup_exam_corpus_dictionary_entry(
    bundle_dir: &Path,
    term: &str,
) -> Result<Option<(String, ExamCorpusDictionaryEntry)>, String> {
    let Some(path) = exam_dictionary_asset_path(bundle_dir) else {
        return Ok(None);
    };
    let cache_key = path.to_string_lossy().to_string();
    let mut cache = EXAM_CORPUS_DICTIONARY_CACHE
        .lock()
        .map_err(|_| "Failed to lock exam corpus dictionary cache".to_string())?;
    if !cache.contains_key(&cache_key) {
        let raw = fs::read_to_string(&path).map_err(|error| {
            format!(
                "Failed to read exam corpus dictionary {}: {error}",
                path.display()
            )
        })?;
        let asset: ExamCorpusDictionaryAsset = serde_json::from_str(&raw).map_err(|error| {
            format!("Invalid exam corpus dictionary {}: {error}", path.display())
        })?;
        cache.insert(
            cache_key.clone(),
            CachedExamCorpusDictionary {
                commit: asset.source.commit,
                entries: asset
                    .entries
                    .into_iter()
                    .map(|entry| (entry.term.trim().to_ascii_lowercase(), entry))
                    .collect(),
            },
        );
    }
    let dictionary = cache
        .get(&cache_key)
        .ok_or_else(|| "Exam corpus dictionary cache disappeared".to_string())?;
    Ok(dictionary
        .entries
        .get(&term.trim().to_ascii_lowercase())
        .cloned()
        .map(|entry| (dictionary.commit.clone(), entry)))
}

fn upsert_exam_corpus_dictionary_entry(
    conn: &word_storage_core::Connection,
    commit: &str,
    entry: &ExamCorpusDictionaryEntry,
) -> Result<i64, String> {
    let source_commit = format!("ecdict-exam-corpus-{commit}");
    conn.execute(
        "INSERT OR IGNORE INTO source_versions
         (source_name, source_commit, status, notes)
         VALUES ('ECDICT/exam-corpus', ?1, 'ready',
                 'ECDICT term selected from bundled exam papers')",
        [&source_commit],
    )
    .map_err(|error| format!("Failed to create exam corpus dictionary source: {error}"))?;
    let source_version_id = conn
        .query_row(
            "SELECT id FROM source_versions WHERE source_commit = ?1",
            [&source_commit],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Failed to load exam corpus dictionary source: {error}"))?;
    let source_entry_key = format!("exam_corpus:{}", entry.term.trim().to_ascii_lowercase());
    conn.execute(
        "INSERT OR IGNORE INTO entries
         (source_version_id, source_entry_key, word, lemma, part_of_speech, frequency)
         VALUES (?1, ?2, ?3, ?4, ?5, 0.9)",
        rusqlite::params![
            source_version_id,
            source_entry_key,
            entry.term.trim(),
            entry.lemma.trim(),
            entry.part_of_speech.trim()
        ],
    )
    .map_err(|error| format!("Failed to create exam corpus entry {}: {error}", entry.term))?;
    let entry_id = conn
        .query_row(
            "SELECT id FROM entries WHERE source_version_id = ?1 AND source_entry_key = ?2",
            rusqlite::params![source_version_id, source_entry_key],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Failed to load exam corpus entry {}: {error}", entry.term))?;
    conn.execute(
        "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
         SELECT ?1, ?2, ?3, 0
         WHERE NOT EXISTS (
             SELECT 1 FROM entry_meanings WHERE entry_id = ?1 AND meaning_cn = ?3
         )",
        rusqlite::params![entry_id, entry.part_of_speech.trim(), entry.meaning.trim()],
    )
    .map_err(|error| {
        format!(
            "Failed to create exam corpus meaning for {}: {error}",
            entry.term
        )
    })?;
    Ok(entry_id)
}

fn ensure_curated_exam_dictionary_entries(
    conn: &word_storage_core::Connection,
) -> Result<(), String> {
    const MAINTENANCE_KEY: &str = "exam_dictionary_entries_v2";
    if get_json_setting(conn, MAINTENANCE_KEY, &serde_json::Value::Bool(false))?
        .as_bool()
        .unwrap_or(false)
    {
        return Ok(());
    }
    const ENTRIES: &[(&str, &str, &str)] = &[
        ("according to", "phrase", "根据；按照"),
        ("as far as", "phrase", "就……而言；达到……程度"),
        ("be experimented upon", "phrase", "被用于实验"),
        ("be related to", "phrase", "与……有关"),
        ("compared with", "phrase", "与……相比"),
        ("end up", "phrase", "最终成为；最终处于"),
        ("for fear that", "phrase", "唯恐；以免"),
        ("in any case", "phrase", "无论如何"),
        ("in case that", "phrase", "万一；如果"),
        ("in store", "phrase", "即将发生；准备着"),
        ("lead to", "phrase", "导致；引起"),
        ("pin down", "phrase", "确切说明；确定"),
        ("slacken off", "phrase", "松懈；减缓"),
        ("so long as", "phrase", "只要"),
        ("turn out", "phrase", "结果是；证明是"),
        ("alleged", "adj", "所谓的；声称的"),
        ("ambiguous", "adj", "含糊的；模棱两可的"),
        ("arise", "v", "产生；出现"),
        ("conduct", "v", "进行；实施"),
        ("controversial", "adj", "有争议的"),
        ("diligent", "adj", "勤奋的；用功的"),
        ("dim", "v", "使变暗；变暗"),
        ("econometric", "adj", "计量经济学的"),
        ("interpretation", "n", "解释；理解"),
        ("mischievous", "adj", "淘气的；恶作剧的"),
        ("peculiar", "adj", "特有的；奇怪的"),
        ("perplexing", "adj", "令人困惑的"),
        ("plateau", "n", "稳定水平；高原"),
        ("productivity", "n", "生产效率；生产力"),
        ("upon", "prep", "在……之上；在……之后"),
    ];
    conn.execute(
        "INSERT OR IGNORE INTO source_versions
         (source_name, source_commit, status, notes)
         VALUES ('word-mobile/exam-phrases', 'exam-phrases-v1', 'ready',
                 'Curated words and multi-word entries used by exam practice')",
        [],
    )
    .map_err(|error| format!("Failed to create exam phrase source: {error}"))?;
    let source_version_id = conn
        .query_row(
            "SELECT id FROM source_versions WHERE source_commit = 'exam-phrases-v1'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Failed to load exam phrase source: {error}"))?;
    for (term, part_of_speech, meaning) in ENTRIES {
        let source_entry_key = format!("exam_supplement:{}", term.replace(' ', "_"));
        conn.execute(
            "INSERT OR IGNORE INTO entries
             (source_version_id, source_entry_key, word, lemma, part_of_speech, frequency)
             VALUES (?1, ?2, ?3, ?3, ?4, 1.0)",
            rusqlite::params![source_version_id, source_entry_key, term, part_of_speech],
        )
        .map_err(|error| format!("Failed to create exam dictionary entry {term}: {error}"))?;
        let entry_id = conn
            .query_row(
                "SELECT id FROM entries
                 WHERE source_version_id = ?1 AND source_entry_key = ?2",
                rusqlite::params![source_version_id, source_entry_key],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|error| format!("Failed to load exam dictionary entry {term}: {error}"))?;
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             SELECT ?1, ?2, ?3, 0
             WHERE NOT EXISTS (
                 SELECT 1 FROM entry_meanings
                 WHERE entry_id = ?1 AND meaning_cn = ?3
             )",
            rusqlite::params![entry_id, part_of_speech, meaning],
        )
        .map_err(|error| format!("Failed to create meaning for {term}: {error}"))?;
    }
    set_json_setting(conn, MAINTENANCE_KEY, &serde_json::Value::Bool(true))
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedExampleOverride {
    #[allow(dead_code)]
    word: Option<String>,
    examples: Vec<SeedExampleOverrideExample>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SeedExampleOverrideExample {
    #[allow(dead_code)]
    pos: Option<String>,
    sentence_en: String,
    sentence_cn: String,
}

fn apply_seed_example_overrides(
    conn: &word_storage_core::Connection,
    bundle_dir: &Path,
) -> Result<(), String> {
    let path = bundle_dir.join("seed-vocab").join("example-overrides.json");
    if !path.exists() {
        return Ok(());
    }
    let content = fs::read_to_string(&path).map_err(|e| {
        format!(
            "Failed to read seed example overrides {}: {e}",
            path.display()
        )
    })?;
    if content.trim().is_empty() {
        return Ok(());
    }
    let overrides: BTreeMap<String, SeedExampleOverride> =
        serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse seed example overrides {}: {e}",
                path.display()
            )
        })?;
    for (source_entry_key, override_entry) in overrides {
        let entry_id = conn
            .query_row(
                "SELECT id FROM entries WHERE source_entry_key = ?1 ORDER BY id ASC LIMIT 1",
                [source_entry_key.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| format!("Failed to query seed override entry {source_entry_key}: {e}"))?;
        let Some(entry_id) = entry_id else {
            continue;
        };
        for example in override_entry.examples {
            insert_seed_entry_example(
                conn,
                entry_id,
                &example.sentence_en,
                &example.sentence_cn,
                None,
            )?;
        }
    }
    Ok(())
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
        let Some(raw_word) = item
            .get("headWord")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let word = item
            .get("displayWord")
            .and_then(|value| value.as_str())
            .or_else(|| {
                item.pointer("/content/word/displayWord")
                    .and_then(|value| value.as_str())
            })
            .or_else(|| {
                item.pointer("/content/word/content/displayWord")
                    .and_then(|value| value.as_str())
            })
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(raw_word);
        let source_key = item
            .pointer("/content/word/wordId")
            .and_then(|value| value.as_str())
            .unwrap_or(raw_word);
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
            .and_then(|value| value.get("trans"))
            .and_then(|value| value.as_array())
            .map(|translations| {
                let mut seen = std::collections::HashSet::new();
                translations
                    .iter()
                    .filter_map(|translation| {
                        translation.get("pos").and_then(|value| value.as_str())
                    })
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .filter(|value| seen.insert((*value).to_string()))
                    .collect::<Vec<_>>()
                    .join(" / ")
            })
            .unwrap_or_default();
        let exam_frequency = word_content
            .and_then(|value| value.pointer("/realExamFrequency/occurrences"))
            .and_then(|value| value.as_i64())
            .unwrap_or(0)
            .max(0);
        let exam_rank = word_content
            .and_then(|value| value.pointer("/realExamFrequency/rank"))
            .and_then(|value| value.as_i64())
            .filter(|value| *value > 0);
        let rank = item
            .get("wordRank")
            .and_then(|value| value.as_i64())
            .unwrap_or(imported as i64 + 1);
        let frequency = 1_000_000.0 - rank.max(0) as f64;

        let canonical_entry_id =
            find_seed_entry_for_word_pos(conn, wordbook_id, word, &part_of_speech)?;
        let (entry_id, should_replace_details) = if let Some(entry_id) = canonical_entry_id {
            conn.execute(
                "UPDATE entries
                 SET frequency = MAX(frequency, ?1),
                     phonetic_us = CASE WHEN COALESCE(phonetic_us, '') = '' THEN ?2 ELSE phonetic_us END,
                     phonetic_uk = CASE WHEN COALESCE(phonetic_uk, '') = '' THEN ?3 ELSE phonetic_uk END,
                     part_of_speech = CASE WHEN ?4 <> '' THEN ?4 ELSE part_of_speech END,
                     exam_frequency = MAX(exam_frequency, ?5),
                     exam_rank = CASE
                         WHEN ?6 IS NULL THEN exam_rank
                         WHEN exam_rank IS NULL THEN ?6
                         ELSE MIN(exam_rank, ?6)
                     END
                 WHERE id = ?7",
                rusqlite::params![frequency, phonetic_us, phonetic_uk, part_of_speech, exam_frequency, exam_rank, entry_id],
            )
            .map_err(|e| format!("Failed to merge seed entry {source_key}: {e}"))?;
            (entry_id, false)
        } else {
            conn.execute(
                "INSERT INTO entries (source_version_id, source_entry_key, word, lemma, phonetic_us, phonetic_uk, part_of_speech, frequency, exam_frequency, exam_rank, difficulty)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, '')
                 ON CONFLICT(source_version_id, source_entry_key) DO UPDATE SET
                    word = excluded.word,
                    lemma = excluded.lemma,
                    phonetic_us = excluded.phonetic_us,
                    phonetic_uk = excluded.phonetic_uk,
                    part_of_speech = excluded.part_of_speech,
                    frequency = excluded.frequency,
                    exam_frequency = excluded.exam_frequency,
                    exam_rank = excluded.exam_rank",
                rusqlite::params![
                    source_version_id,
                    source_key,
                    word,
                    word.to_lowercase(),
                    phonetic_us,
                    phonetic_uk,
                    part_of_speech,
                    frequency,
                    exam_frequency,
                    exam_rank,
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
        persist_seed_entry_precomputed_question_preps(conn, entry_id, &item)?;
        persist_seed_entry_aliases(conn, entry_id, word_content)?;
        imported += 1;
    }

    Ok(imported)
}

fn persist_seed_entry_aliases(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    word_content: Option<&serde_json::Value>,
) -> Result<(), String> {
    let relations = word_content
        .and_then(|value| value.pointer("/relWord/rels"))
        .and_then(|value| value.as_array());
    for relation in relations.into_iter().flatten() {
        let Some(words) = relation.get("words").and_then(|value| value.as_array()) else {
            continue;
        };
        for word in words {
            let alias = word
                .get("hwd")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .unwrap_or("");
            if alias.is_empty() {
                continue;
            }
            let meaning = word
                .get("tran")
                .and_then(|value| value.as_str())
                .map(clean_seed_meaning_cn)
                .unwrap_or_default();
            conn.execute(
                "INSERT OR IGNORE INTO entry_aliases
                    (entry_id, alias, meaning_cn, source_kind)
                 VALUES (?1, ?2, ?3, 'related_word')",
                rusqlite::params![entry_id, alias, meaning],
            )
            .map_err(|error| format!("Failed to persist seed alias {alias}: {error}"))?;
        }
    }
    let synonyms = word_content
        .and_then(|value| value.pointer("/syno/synos"))
        .and_then(|value| value.as_array());
    for synonym_group in synonyms.into_iter().flatten() {
        let meaning = synonym_group
            .get("tran")
            .and_then(|value| value.as_str())
            .map(clean_seed_meaning_cn)
            .unwrap_or_default();
        if meaning.is_empty() {
            continue;
        }
        let Some(words) = synonym_group.get("hwds").and_then(|value| value.as_array()) else {
            continue;
        };
        for word in words {
            let alias = word
                .get("w")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .unwrap_or("");
            if alias.is_empty() {
                continue;
            }
            conn.execute(
                "INSERT OR IGNORE INTO entry_aliases
                    (entry_id, alias, meaning_cn, source_kind)
                 VALUES (?1, ?2, ?3, 'synonym')",
                rusqlite::params![entry_id, alias, meaning],
            )
            .map_err(|error| format!("Failed to persist seed synonym {alias}: {error}"))?;
        }
    }
    Ok(())
}

fn import_seed_aliases(
    conn: &word_storage_core::Connection,
    book_dir: &Path,
) -> Result<(), String> {
    for entry in fs::read_dir(book_dir)
        .map_err(|error| format!("Failed to read seed alias assets: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("Failed to inspect seed alias asset: {error}"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let content = fs::read_to_string(&path).map_err(|error| {
            format!(
                "Failed to read seed alias asset {}: {error}",
                path.display()
            )
        })?;
        let items: Vec<serde_json::Value> = serde_json::from_str(&content).map_err(|error| {
            format!(
                "Failed to parse seed alias asset {}: {error}",
                path.display()
            )
        })?;
        for item in items {
            let source_key = item
                .pointer("/content/word/wordId")
                .and_then(|value| value.as_str());
            let head_word = item.get("headWord").and_then(|value| value.as_str());
            let entry_id = source_key
                .and_then(|key| {
                    conn.query_row(
                        "SELECT id FROM entries WHERE source_entry_key = ?1 ORDER BY id LIMIT 1",
                        [key],
                        |row| row.get::<_, i64>(0),
                    )
                    .optional()
                    .ok()
                    .flatten()
                })
                .or_else(|| {
                    head_word.and_then(|word| {
                        conn.query_row(
                            "SELECT id FROM entries WHERE word = ?1 COLLATE NOCASE ORDER BY frequency DESC, id LIMIT 1",
                            [word],
                            |row| row.get::<_, i64>(0),
                        )
                        .optional()
                        .ok()
                        .flatten()
                    })
                });
            if let Some(entry_id) = entry_id {
                persist_seed_entry_aliases(conn, entry_id, item.pointer("/content/word/content"))?;
            }
        }
    }
    Ok(())
}

fn refresh_seed_precomputed_question_preps(
    conn: &word_storage_core::Connection,
    book_dir: &Path,
) -> Result<usize, String> {
    let books = [
        (1i64, "CET4_3.json"),
        (2i64, "CET6_3.json"),
        (3i64, "KaoYan_3.json"),
        (4i64, "MEDICAL_RESP.json"),
    ];
    let mut refreshed = 0usize;
    for (wordbook_id, file_name) in books {
        let path = book_dir.join(file_name);
        if !path.exists() {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read seed question preps {}: {e}", path.display()))?;
        let items: Vec<serde_json::Value> = serde_json::from_str(&content).map_err(|e| {
            format!(
                "Failed to parse seed question preps {}: {e}",
                path.display()
            )
        })?;
        for item in items {
            let Some(raw_word) = item
                .get("headWord")
                .and_then(|value| value.as_str())
                .or_else(|| {
                    item.pointer("/content/word/wordHead")
                        .and_then(|value| value.as_str())
                })
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let word = item
                .get("displayWord")
                .and_then(|value| value.as_str())
                .or_else(|| {
                    item.pointer("/content/word/displayWord")
                        .and_then(|value| value.as_str())
                })
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(raw_word);
            let part_of_speech = item
                .pointer("/content/word/content/trans/0/pos")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let Some(entry_id) =
                find_seed_entry_for_word_pos(conn, wordbook_id, word, part_of_speech)?
            else {
                continue;
            };
            let has_precomputed = read_seed_choice_distractors(&item, "cnChoiceDistractors")
                .or_else(|| read_seed_choice_distractors(&item, "cn_choice_distractors"))
                .is_some_and(|values| !values.is_empty())
                || read_seed_choice_distractors(&item, "enChoiceDistractors")
                    .or_else(|| read_seed_choice_distractors(&item, "en_choice_distractors"))
                    .is_some_and(|values| !values.is_empty());
            if !has_precomputed {
                continue;
            }
            persist_seed_entry_precomputed_question_preps(conn, entry_id, &item)?;
            refreshed += 1;
        }
    }
    Ok(refreshed)
}

fn persist_seed_entry_precomputed_question_preps(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    item: &serde_json::Value,
) -> Result<(), String> {
    let cn_choice_distractors = read_seed_choice_distractors(item, "cnChoiceDistractors")
        .or_else(|| read_seed_choice_distractors(item, "cn_choice_distractors"))
        .unwrap_or_default();
    let en_choice_distractors = read_seed_choice_distractors(item, "enChoiceDistractors")
        .or_else(|| read_seed_choice_distractors(item, "en_choice_distractors"))
        .unwrap_or_default();
    if cn_choice_distractors.is_empty() && en_choice_distractors.is_empty() {
        return Ok(());
    }

    conn.execute(
        "INSERT OR REPLACE INTO entry_question_preps
            (entry_id, cn_choice_distractors_json, en_choice_distractors_json, updated_at)
         VALUES (?1, ?2, ?3, datetime('now'))",
        rusqlite::params![
            entry_id,
            serde_json::to_string(&cn_choice_distractors)
                .map_err(|e| format!("Failed to serialize seed CN question preps: {e}"))?,
            serde_json::to_string(&en_choice_distractors)
                .map_err(|e| format!("Failed to serialize seed EN question preps: {e}"))?,
        ],
    )
    .map_err(|e| format!("Failed to persist seed question preps: {e}"))?;
    Ok(())
}

fn read_seed_choice_distractors(item: &serde_json::Value, field: &str) -> Option<Vec<String>> {
    let word_pointer = format!("/content/word/{field}");
    let content_pointer = format!("/content/word/content/{field}");
    let values = item
        .get(field)
        .or_else(|| item.pointer(&word_pointer))
        .or_else(|| item.pointer(&content_pointer))
        .and_then(|value| value.as_array())?;
    let out = values
        .iter()
        .filter_map(|value| value.as_str())
        .map(clean_seed_meaning_cn)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    Some(out)
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
        let sentence_cn = example
            .get("sCn")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        insert_seed_entry_example(conn, entry_id, sentence_en, sentence_cn, Some(index as i64))?;
    }
    append_seed_entry_phrase_examples(conn, entry_id, word_content)?;
    append_seed_entry_real_exam_examples(conn, entry_id, word_content)?;
    Ok(())
}

fn append_seed_entry_phrase_examples(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    word_content: Option<&serde_json::Value>,
) -> Result<(), String> {
    let phrases = word_content
        .and_then(|value| value.pointer("/phrase/phrases"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    for phrase in phrases.iter() {
        let phrase_en = phrase
            .get("pContent")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let phrase_cn = phrase
            .get("pCn")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        if phrase_en.is_empty() || phrase_cn.is_empty() {
            continue;
        }
        insert_seed_entry_example(conn, entry_id, phrase_en, phrase_cn, None)?;
    }
    Ok(())
}

fn append_seed_entry_real_exam_examples(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    word_content: Option<&serde_json::Value>,
) -> Result<(), String> {
    let examples = word_content
        .and_then(|value| value.pointer("/realExamSentence/sentences"))
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    for example in examples.iter() {
        let sentence_en = example
            .get("sContent")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        if sentence_en.is_empty() {
            continue;
        }
        let sentence_cn = example
            .get("sCn")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        insert_seed_entry_example(conn, entry_id, sentence_en, sentence_cn, None)?;
    }
    Ok(())
}

fn insert_seed_entry_example(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    sentence_en: &str,
    sentence_cn: &str,
    preferred_sort_order: Option<i64>,
) -> Result<(), String> {
    let sentence_en = sentence_en.trim();
    if sentence_en.is_empty() {
        return Ok(());
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
        return Ok(());
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
            sentence_cn,
            preferred_sort_order
                .map(|value| sort_order.max(value))
                .unwrap_or(sort_order),
        ],
    )
    .map_err(|e| format!("Failed to insert seed entry example: {e}"))?;
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

#[derive(Debug, Clone)]
struct EntryQuestionPrepCandidate {
    wordbook_id: i64,
    entry_id: i64,
    word: String,
    pos_key: String,
    meaning_cn: String,
    meaning_key: String,
    rank_in_book: i64,
}

fn rebuild_seed_question_preps(conn: &word_storage_core::Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT we.wordbook_id, e.id, e.word, COALESCE(e.part_of_speech, ''), em.pos, em.meaning_cn, we.rank_in_book
             FROM entries e
             JOIN wordbook_entries we ON we.entry_id = e.id
             LEFT JOIN entry_meanings em ON em.entry_id = e.id
             ORDER BY we.wordbook_id ASC, we.rank_in_book ASC, em.sort_order ASC",
        )
        .map_err(|e| format!("Failed to prepare question prep source query: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let entry_pos: Option<String> = row.get(3)?;
            let meaning_pos: Option<String> = row.get(4)?;
            let raw_meaning: Option<String> = row.get(5)?;
            let meaning_cn = clean_seed_meaning_cn(raw_meaning.as_deref().unwrap_or(""));
            let raw_pos = meaning_pos
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .or(entry_pos.as_deref())
                .unwrap_or("");
            Ok(EntryQuestionPrepCandidate {
                wordbook_id: row.get(0)?,
                entry_id: row.get(1)?,
                word: row.get(2)?,
                pos_key: normalize_seed_pos_key(raw_pos),
                meaning_key: normalize_seed_meaning_for_comparison(&meaning_cn),
                meaning_cn,
                rank_in_book: row.get(6)?,
            })
        })
        .map_err(|e| format!("Failed to query question prep source rows: {e}"))?;

    let mut by_entry: BTreeMap<i64, EntryQuestionPrepCandidate> = BTreeMap::new();
    let mut by_wordbook: BTreeMap<i64, Vec<EntryQuestionPrepCandidate>> = BTreeMap::new();
    for row in rows {
        let candidate = row.map_err(|e| format!("Failed to read question prep source row: {e}"))?;
        if candidate.meaning_cn.is_empty() || candidate.meaning_key.is_empty() {
            continue;
        }
        by_entry
            .entry(candidate.entry_id)
            .or_insert_with(|| candidate.clone());
        by_wordbook
            .entry(candidate.wordbook_id)
            .or_default()
            .push(candidate);
    }

    for target in by_entry.values() {
        let same_book_entries = by_wordbook
            .get(&target.wordbook_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut ranked = same_book_entries
            .iter()
            .filter(|candidate| candidate.entry_id != target.entry_id)
            .filter(|candidate| !target.pos_key.is_empty() && candidate.pos_key == target.pos_key)
            .filter(|candidate| candidate.meaning_key != target.meaning_key)
            .map(|candidate| {
                let score = seed_text_overlap_score(&target.meaning_cn, &candidate.meaning_cn);
                let distance = (candidate.rank_in_book - target.rank_in_book).abs();
                (
                    std::cmp::Reverse(score),
                    distance,
                    candidate.entry_id,
                    candidate,
                )
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then(left.1.cmp(&right.1))
                .then(left.2.cmp(&right.2))
        });

        let mut cn_choice_distractors = Vec::new();
        let mut en_choice_distractors = Vec::new();
        let mut seen_cn = BTreeSet::new();
        let mut seen_en = BTreeSet::new();
        for (_, _, _, candidate) in ranked {
            if cn_choice_distractors.len() < 7 && seen_cn.insert(candidate.meaning_key.clone()) {
                cn_choice_distractors.push(candidate.meaning_cn.clone());
            }
            let word_key = candidate.word.to_ascii_lowercase();
            if en_choice_distractors.len() < 7 && seen_en.insert(word_key) {
                en_choice_distractors.push(candidate.word.clone());
            }
            if cn_choice_distractors.len() >= 7 && en_choice_distractors.len() >= 7 {
                break;
            }
        }

        conn.execute(
            "INSERT OR REPLACE INTO entry_question_preps
                (entry_id, cn_choice_distractors_json, en_choice_distractors_json, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            rusqlite::params![
                target.entry_id,
                serde_json::to_string(&cn_choice_distractors)
                    .map_err(|e| format!("Failed to serialize CN question preps: {e}"))?,
                serde_json::to_string(&en_choice_distractors)
                    .map_err(|e| format!("Failed to serialize EN question preps: {e}"))?,
            ],
        )
        .map_err(|e| {
            format!(
                "Failed to save question preps for entry {}: {e}",
                target.entry_id
            )
        })?;
    }

    Ok(())
}
fn clean_seed_meaning_cn(value: &str) -> String {
    let without_markers = value.replace(['<', '>'], "");
    let normalized = without_markers
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    strip_seed_exam_markers(&normalized)
}

fn strip_seed_exam_markers(value: &str) -> String {
    let parts = value
        .split([';', ','])
        .map(|part| {
            part.trim()
                .trim_end_matches(|ch| matches!(ch, 'A' | 'B' | 'C' | 'D'))
                .trim()
        })
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        String::new()
    } else {
        parts.join(", ")
    }
}

fn has_seed_exam_marker(value: &str) -> bool {
    value
        .split([';', ','])
        .any(|part| matches!(part.trim().chars().last(), Some('A' | 'B' | 'C' | 'D')))
}

fn clean_seed_meaning_text_if_needed(text: &str) -> String {
    if text.contains('<') || text.contains('>') || has_seed_exam_marker(text) {
        clean_seed_meaning_cn(text)
    } else {
        text.to_string()
    }
}

fn clean_seed_meaning_noise_in_json(value: &mut serde_json::Value) {
    clean_seed_meaning_noise_in_json_field(value, None);
}

fn clean_seed_meaning_noise_in_json_field(value: &mut serde_json::Value, field_name: Option<&str>) {
    match value {
        serde_json::Value::String(text) => {
            if field_allows_seed_meaning_cleanup(field_name) {
                let cleaned = clean_seed_meaning_text_if_needed(text);
                if cleaned != *text {
                    *text = cleaned;
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                clean_seed_meaning_noise_in_json_field(item, field_name);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                clean_seed_meaning_noise_in_json_field(child, Some(key.as_str()));
            }
        }
        _ => {}
    }
}

fn field_allows_seed_meaning_cleanup(field_name: Option<&str>) -> bool {
    let Some(field_name) = field_name else {
        return false;
    };
    matches!(
        field_name,
        "meaningCn"
            | "meaning_cn"
            | "meanings"
            | "acceptedMeanings"
            | "correctAnswer"
            | "exampleTranslation"
            | "sentenceCn"
            | "sentence_cn"
            | "preview"
            | "text"
    )
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
    const SEED_MEANING_NOISE_REPAIR_KEY: &str = "seed_meaning_noise_repair_v1";
    let repair_done = get_json_setting(
        conn,
        SEED_MEANING_NOISE_REPAIR_KEY,
        &serde_json::Value::Bool(false),
    )?
    .as_bool()
    .unwrap_or(false);
    if repair_done {
        return Ok(());
    }

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
    set_json_setting(
        conn,
        SEED_MEANING_NOISE_REPAIR_KEY,
        &serde_json::Value::Bool(true),
    )?;

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
    let targets = today_target_seed_from_plan_value_for_date(plan_value, today_date);
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

fn load_reward_image_upload_entitlement(
    conn: &word_storage_core::Connection,
) -> Result<serde_json::Value, String> {
    let row = conn
        .query_row(
            "SELECT owner_key, available_uploads, last_granted_streak_milestone, updated_at
             FROM reward_image_upload_entitlements
             WHERE owner_key = 'local'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|e| format!("Failed to load reward image entitlement: {e}"))?;

    let (owner_key, available_uploads, last_granted_streak_milestone, updated_at) =
        row.unwrap_or_else(|| ("local".to_owned(), 0, 0, String::new()));
    Ok(serde_json::json!({
        "ownerKey": owner_key,
        "availableUploads": available_uploads,
        "lastGrantedStreakMilestone": last_granted_streak_milestone,
        "nextMilestoneStreakDays": (last_granted_streak_milestone + 1) * 5,
        "updatedAt": updated_at,
    }))
}

fn refresh_reward_image_entitlement_with_connection(
    conn: &word_storage_core::Connection,
    current_streak_days: i64,
    max_available_uploads: i64,
) -> Result<serde_json::Value, String> {
    let milestone = current_streak_days.max(0) / 5;
    let max_available_uploads = max_available_uploads.max(1);
    let current = load_reward_image_upload_entitlement(conn)?;
    let previous_milestone = current
        .get("lastGrantedStreakMilestone")
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let previous_available = current
        .get("availableUploads")
        .and_then(|value| value.as_i64())
        .unwrap_or(0);
    let newly_granted = (milestone - previous_milestone).max(0);
    let available_uploads = (previous_available + newly_granted).min(max_available_uploads);
    let stored_milestone = previous_milestone.max(milestone);

    conn.execute(
        "INSERT INTO reward_image_upload_entitlements (
            owner_key,
            available_uploads,
            last_granted_streak_milestone,
            updated_at
        )
        VALUES ('local', ?1, ?2, datetime('now'))
        ON CONFLICT(owner_key) DO UPDATE SET
            available_uploads = excluded.available_uploads,
            last_granted_streak_milestone = excluded.last_granted_streak_milestone,
            updated_at = datetime('now')",
        rusqlite::params![available_uploads, stored_milestone],
    )
    .map_err(|e| format!("Failed to refresh reward image entitlement: {e}"))?;

    load_reward_image_upload_entitlement(conn)
}

fn create_reward_image_upload_with_connection(
    conn: &word_storage_core::Connection,
    local_path: &str,
    mime_type: &str,
    original_filename: &str,
) -> Result<serde_json::Value, String> {
    conn.execute(
        "INSERT INTO reward_images (
            owner_key,
            local_path,
            mime_type,
            original_filename,
            moderation_status,
            created_at,
            updated_at
        )
        VALUES ('local', ?1, ?2, ?3, 'pending', datetime('now'), datetime('now'))",
        rusqlite::params![local_path, mime_type, original_filename],
    )
    .map_err(|e| format!("Failed to create reward image upload: {e}"))?;
    let image_id = conn.last_insert_rowid();
    reward_image_by_id(conn, image_id)
}

fn list_reward_images_with_connection(
    conn: &word_storage_core::Connection,
    public_only: bool,
    week_start: &str,
) -> Result<serde_json::Value, String> {
    let sql = if public_only {
        "SELECT id, owner_key, local_path, mime_type, original_filename, moderation_status,
            moderation_reason, moderation_checked_at, is_withdrawn, draw_pool_eligible,
            created_at, updated_at
         FROM reward_images
         WHERE moderation_status = 'approved' AND is_withdrawn = 0
         ORDER BY created_at DESC"
    } else {
        "SELECT id, owner_key, local_path, mime_type, original_filename, moderation_status,
            moderation_reason, moderation_checked_at, is_withdrawn, draw_pool_eligible,
            created_at, updated_at
         FROM reward_images
         WHERE owner_key = 'local'
         ORDER BY created_at DESC"
    };
    let mut statement = conn
        .prepare(sql)
        .map_err(|e| format!("Failed to prepare reward image query: {e}"))?;
    let rows = statement
        .query_map([], |row| reward_image_from_row(conn, row, week_start))
        .map_err(|e| format!("Failed to query reward images: {e}"))?;
    let images = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read reward image: {e}"))?;
    Ok(serde_json::json!({ "images": images }))
}

fn moderate_reward_image_with_connection(
    conn: &word_storage_core::Connection,
    image_id: i64,
    status: &str,
    reason: &str,
) -> Result<serde_json::Value, String> {
    if !matches!(status, "pending" | "approved" | "rejected") {
        return Err("status must be pending, approved, or rejected".to_string());
    }
    let changed = conn
        .execute(
            "UPDATE reward_images
             SET moderation_status = ?1,
                 moderation_reason = ?2,
                 moderation_checked_at = CASE WHEN ?1 = 'pending' THEN NULL ELSE datetime('now') END,
                 updated_at = datetime('now')
             WHERE id = ?3 AND owner_key = 'local'",
            rusqlite::params![status, reason, image_id],
        )
        .map_err(|e| format!("Failed to update reward image moderation: {e}"))?;
    if changed == 0 {
        return Err("Reward image was not found".to_string());
    }
    reward_image_by_id(conn, image_id)
}

fn vote_reward_image_with_connection(
    conn: &word_storage_core::Connection,
    image_id: i64,
    voter_key: &str,
    week_start: &str,
) -> Result<serde_json::Value, String> {
    let image = reward_image_by_id(conn, image_id)?;
    if image
        .get("moderationStatus")
        .and_then(|value| value.as_str())
        != Some("approved")
    {
        return Err("Only approved images can receive votes".to_string());
    }
    let inserted = conn
        .execute(
            "INSERT OR IGNORE INTO reward_image_votes (image_id, voter_key, week_start, created_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            rusqlite::params![image_id, voter_key, week_start],
        )
        .map_err(|e| format!("Failed to vote for reward image: {e}"))?;
    let vote_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM reward_image_votes WHERE image_id = ?1 AND week_start = ?2",
            rusqlite::params![image_id, week_start],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count reward image votes: {e}"))?;
    Ok(serde_json::json!({
        "imageId": image_id,
        "weekStart": week_start,
        "inserted": inserted > 0,
        "voteCount": vote_count,
    }))
}

fn reward_image_by_id(
    conn: &word_storage_core::Connection,
    image_id: i64,
) -> Result<serde_json::Value, String> {
    conn.query_row(
        "SELECT id, owner_key, local_path, mime_type, original_filename, moderation_status,
            moderation_reason, moderation_checked_at, is_withdrawn, draw_pool_eligible,
            created_at, updated_at
         FROM reward_images
         WHERE id = ?1",
        rusqlite::params![image_id],
        |row| reward_image_from_row(conn, row, &current_week_start_string()),
    )
    .optional()
    .map_err(|e| format!("Failed to load reward image: {e}"))?
    .ok_or_else(|| "Reward image was not found".to_string())
}

fn reward_image_from_row(
    conn: &word_storage_core::Connection,
    row: &rusqlite::Row<'_>,
    week_start: &str,
) -> rusqlite::Result<serde_json::Value> {
    let image_id = row.get::<_, i64>(0)?;
    let vote_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM reward_image_votes WHERE image_id = ?1 AND week_start = ?2",
            rusqlite::params![image_id, week_start],
            |row| row.get(0),
        )
        .unwrap_or(0);
    let selected_as_tag = conn
        .query_row(
            "SELECT 1 FROM leaderboard_image_tags WHERE owner_key = 'local' AND image_id = ?1",
            rusqlite::params![image_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    Ok(serde_json::json!({
        "id": image_id,
        "ownerKey": row.get::<_, String>(1)?,
        "localPath": row.get::<_, String>(2)?,
        "mimeType": row.get::<_, String>(3)?,
        "originalFilename": row.get::<_, String>(4)?,
        "moderationStatus": row.get::<_, String>(5)?,
        "moderationReason": row.get::<_, String>(6)?,
        "moderationCheckedAt": row.get::<_, Option<String>>(7)?,
        "isWithdrawn": row.get::<_, i64>(8)? != 0,
        "drawPoolEligible": row.get::<_, i64>(9)? != 0,
        "createdAt": row.get::<_, String>(10)?,
        "updatedAt": row.get::<_, String>(11)?,
        "voteCount": vote_count,
        "selectedAsTag": selected_as_tag,
    }))
}

struct LocalLeaderboardSummaryInput<'a> {
    user_key: &'a str,
    display_name: &'a str,
    period: &'a str,
    period_start: &'a str,
    total_questions: i64,
    correct_count: i64,
    mixed_test_total_questions: i64,
    mixed_test_correct_count: i64,
    current_streak_days: i64,
}

fn upsert_local_leaderboard_summary_with_connection(
    conn: &word_storage_core::Connection,
    input: LocalLeaderboardSummaryInput<'_>,
) -> Result<serde_json::Value, String> {
    if input.user_key.trim().is_empty() {
        return Err("userKey is required".to_string());
    }
    if !matches!(input.period, "weekly" | "monthly" | "all_time") {
        return Err("period must be weekly, monthly, or all_time".to_string());
    }
    conn.execute(
        "INSERT INTO local_leaderboard_summaries (
            user_key,
            display_name,
            total_questions,
            correct_count,
            mixed_test_total_questions,
            mixed_test_correct_count,
            current_streak_days,
            period,
            period_start,
            updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))
        ON CONFLICT(user_key, period, period_start) DO UPDATE SET
            display_name = excluded.display_name,
            total_questions = excluded.total_questions,
            correct_count = excluded.correct_count,
            mixed_test_total_questions = excluded.mixed_test_total_questions,
            mixed_test_correct_count = excluded.mixed_test_correct_count,
            current_streak_days = excluded.current_streak_days,
            updated_at = datetime('now')",
        rusqlite::params![
            input.user_key,
            input.display_name,
            input.total_questions,
            input.correct_count,
            input.mixed_test_total_questions,
            input.mixed_test_correct_count,
            input.current_streak_days,
            input.period,
            input.period_start,
        ],
    )
    .map_err(|e| format!("Failed to refresh local leaderboard summary: {e}"))?;

    local_leaderboard_entry_by_key(conn, input.user_key, input.period, input.period_start, 1)
}

fn get_local_leaderboard_with_connection(
    conn: &word_storage_core::Connection,
    metric: &str,
    period: &str,
    period_start: &str,
    limit: i64,
) -> Result<serde_json::Value, String> {
    let query_period = if metric == "currentStreak" {
        "all_time"
    } else {
        period
    };
    let query_period_start = if query_period == "all_time" {
        "1970-01-01"
    } else {
        period_start
    };
    let order_by = match metric {
        "accuracy" => {
            "CASE WHEN total_questions > 0 THEN 1.0 * correct_count / total_questions ELSE 0 END DESC, total_questions DESC"
        }
        "mixedAccuracy" => {
            "CASE WHEN mixed_test_total_questions > 0 THEN 1.0 * mixed_test_correct_count / mixed_test_total_questions ELSE 0 END DESC, mixed_test_total_questions DESC"
        }
        "currentStreak" => "current_streak_days DESC, total_questions DESC",
        _ => "total_questions DESC, correct_count DESC",
    };
    let sql = format!(
        "SELECT user_key, display_name, total_questions, correct_count,
            mixed_test_total_questions, mixed_test_correct_count,
            current_streak_days, updated_at
         FROM local_leaderboard_summaries
         WHERE period = ?1 AND period_start = ?2
         ORDER BY {order_by}, updated_at ASC
         LIMIT ?3"
    );
    let mut statement = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare local leaderboard query: {e}"))?;
    let mut rank = 0_i64;
    let rows = statement
        .query_map(
            rusqlite::params![query_period, query_period_start, limit],
            |row| {
                rank += 1;
                local_leaderboard_entry_from_row(conn, row, rank)
            },
        )
        .map_err(|e| format!("Failed to query local leaderboard: {e}"))?;
    let entries = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read local leaderboard row: {e}"))?;
    Ok(serde_json::json!({ "entries": entries }))
}

fn local_leaderboard_entry_by_key(
    conn: &word_storage_core::Connection,
    user_key: &str,
    period: &str,
    period_start: &str,
    rank: i64,
) -> Result<serde_json::Value, String> {
    conn.query_row(
        "SELECT user_key, display_name, total_questions, correct_count,
            mixed_test_total_questions, mixed_test_correct_count,
            current_streak_days, updated_at
         FROM local_leaderboard_summaries
         WHERE user_key = ?1 AND period = ?2 AND period_start = ?3",
        rusqlite::params![user_key, period, period_start],
        |row| local_leaderboard_entry_from_row(conn, row, rank),
    )
    .optional()
    .map_err(|e| format!("Failed to load local leaderboard entry: {e}"))?
    .ok_or_else(|| "Local leaderboard entry was not found".to_string())
}

fn local_leaderboard_entry_from_row(
    conn: &word_storage_core::Connection,
    row: &rusqlite::Row<'_>,
    rank: i64,
) -> rusqlite::Result<serde_json::Value> {
    let user_key: String = row.get(0)?;
    let total_questions: i64 = row.get(2)?;
    let correct_count: i64 = row.get(3)?;
    let mixed_total: i64 = row.get(4)?;
    let mixed_correct: i64 = row.get(5)?;
    let accuracy = if total_questions > 0 {
        (correct_count as f64) * 100.0 / (total_questions as f64)
    } else {
        0.0
    };
    let mixed_accuracy = if mixed_total > 0 {
        (mixed_correct as f64) * 100.0 / (mixed_total as f64)
    } else {
        0.0
    };
    let image = selected_reward_image_for_owner(conn, &user_key)
        .ok()
        .flatten();
    Ok(serde_json::json!({
        "rank": rank,
        "is_current_user": user_key == "local",
        "user_id": user_key,
        "display_name": row.get::<_, String>(1)?,
        "total_questions": total_questions,
        "correct_count": correct_count,
        "accuracy_percent": accuracy,
        "mixed_test_total_questions": mixed_total,
        "mixed_test_correct_count": mixed_correct,
        "mixed_test_accuracy_percent": mixed_accuracy,
        "current_streak_days": row.get::<_, i64>(6)?,
        "updated_at": row.get::<_, String>(7)?,
        "tag_image": image,
    }))
}

fn selected_reward_image_for_owner(
    conn: &word_storage_core::Connection,
    owner_key: &str,
) -> Result<Option<serde_json::Value>, String> {
    let image_id: Option<i64> = conn
        .query_row(
            "SELECT image_id FROM leaderboard_image_tags WHERE owner_key = ?1",
            rusqlite::params![owner_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to load local leaderboard tag: {e}"))?;
    let Some(image_id) = image_id else {
        return Ok(None);
    };
    let image = reward_image_by_id(conn, image_id)?;
    if image
        .get("moderationStatus")
        .and_then(|value| value.as_str())
        == Some("approved")
    {
        Ok(Some(image))
    } else {
        Ok(None)
    }
}

const DEMO_REWARD_IMAGE_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x04, 0x00, 0x00, 0x00, 0xB5, 0x1C, 0x0C,
    0x02, 0x00, 0x00, 0x00, 0x0B, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0xF8, 0x0F, 0x04, 0x00,
    0x09, 0xFB, 0x03, 0xFD, 0xA7, 0x05, 0xDD, 0xC1, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44,
    0xAE, 0x42, 0x60, 0x82,
];

fn seed_local_leaderboard_demo_with_connection(
    conn: &word_storage_core::Connection,
    image_dir: Option<&Path>,
) -> Result<serde_json::Value, String> {
    let week_start = current_week_start_string();
    let month_start = {
        use chrono::{Datelike, Local};
        let today = Local::now().date_naive();
        format!("{:04}-{:02}-01", today.year(), today.month())
    };

    for (period, period_start) in [
        ("weekly", week_start.as_str()),
        ("monthly", month_start.as_str()),
        ("all_time", "1970-01-01"),
    ] {
        upsert_local_leaderboard_summary_with_connection(
            conn,
            LocalLeaderboardSummaryInput {
                user_key: "demo_learner",
                display_name: "Local Demo Learner",
                period,
                period_start,
                total_questions: 168,
                correct_count: 142,
                mixed_test_total_questions: 48,
                mixed_test_correct_count: 41,
                current_streak_days: 6,
            },
        )?;
    }

    let local_path = if let Some(image_dir) = image_dir {
        fs::create_dir_all(image_dir)
            .map_err(|e| format!("Failed to create demo reward image directory: {e}"))?;
        let image_path = image_dir.join("local_demo_reward_image.png");
        if !image_path.exists() {
            fs::write(&image_path, DEMO_REWARD_IMAGE_PNG)
                .map_err(|e| format!("Failed to write demo reward image: {e}"))?;
        }
        image_path.display().to_string()
    } else {
        "local_demo_reward_image.png".to_string()
    };
    let image = match conn
        .query_row(
            "SELECT id FROM reward_images WHERE owner_key = 'demo_learner' AND local_path = ?1",
            rusqlite::params![local_path.as_str()],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| format!("Failed to find demo reward image: {e}"))?
    {
        Some(image_id) => reward_image_by_id(conn, image_id)?,
        None => {
            conn.execute(
                "INSERT INTO reward_images (
                    owner_key,
                    local_path,
                    mime_type,
                    original_filename,
                    moderation_status,
                    moderation_checked_at,
                    created_at,
                    updated_at
                )
                VALUES ('demo_learner', ?1, 'image/png', 'demo_reward_image.png', 'approved', datetime('now'), datetime('now'), datetime('now'))",
                rusqlite::params![local_path.as_str()],
            )
            .map_err(|e| format!("Failed to create demo reward image: {e}"))?;
            reward_image_by_id(conn, conn.last_insert_rowid())?
        }
    };
    let image_id = image
        .get("id")
        .and_then(|value| value.as_i64())
        .ok_or("Demo image id was missing")?;
    conn.execute(
        "INSERT INTO leaderboard_image_tags (owner_key, image_id, updated_at)
         VALUES ('demo_learner', ?1, datetime('now'))
         ON CONFLICT(owner_key) DO UPDATE SET image_id = excluded.image_id, updated_at = excluded.updated_at",
        rusqlite::params![image_id],
    )
    .map_err(|e| format!("Failed to select demo leaderboard tag: {e}"))?;

    let leaderboard =
        get_local_leaderboard_with_connection(conn, "totalQuestions", "weekly", &week_start, 50)?;
    let vote = vote_reward_image_with_connection(conn, image_id, "local", &week_start)?;
    let images = list_reward_images_with_connection(conn, true, &week_start)?;
    Ok(serde_json::json!({
        "demoUserKey": "demo_learner",
        "streakDays": 6,
        "imageId": image_id,
        "leaderboard": leaderboard,
        "vote": vote,
        "images": images,
    }))
}

fn current_week_start_string() -> String {
    use chrono::{Datelike, Duration, Local};

    let today = Local::now().date_naive();
    let offset = today.weekday().num_days_from_monday() as i64;
    (today - Duration::days(offset))
        .format("%Y-%m-%d")
        .to_string()
}

fn today_target_seed_from_plan_value(plan_value: &serde_json::Value) -> TodayTargetSeed {
    let today = today_date_string();
    today_target_seed_from_plan_value_for_date(plan_value, &today)
}

fn today_target_seed_from_plan_value_for_date(
    plan_value: &serde_json::Value,
    today_date: &str,
) -> TodayTargetSeed {
    let new_word_questions =
        new_word_question_count_from_plan(plan_value, "newWordsPerDay", today_date);
    let review_questions =
        grown_plan_unit_count(plan_value, "review", "reviewWordsPerDay", today_date);
    let mixed_test = grown_plan_unit_count(plan_value, "mixedTest", "mixedTestPerDay", today_date);
    let wrong_word_test = grown_plan_unit_count(
        plan_value,
        "wrongWordReinforcement",
        "wrongWordTestPerDay",
        today_date,
    );
    let high_frequency = grown_plan_unit_count(
        plan_value,
        "highFrequency",
        "highFrequencyPerDay",
        today_date,
    );
    let root_affix = grown_plan_unit_count(plan_value, "rootAffix", "rootAffixPerDay", today_date);

    TodayTargetSeed {
        new_words_target: Some(new_word_questions),
        new_words_base_target: Some(new_word_questions),
        new_words_carryover_target: Some(0),
        review_words_target: Some(review_questions),
        review_words_base_target: Some(review_questions),
        review_words_carryover_target: Some(0),
        mixed_test_target: Some(mixed_test),
        mixed_test_base_target: Some(mixed_test),
        mixed_test_carryover_target: Some(0),
        wrong_word_test_target: Some(wrong_word_test),
        wrong_word_test_base_target: Some(wrong_word_test),
        wrong_word_test_carryover_target: Some(0),
        high_frequency_target: Some(high_frequency),
        high_frequency_base_target: Some(high_frequency),
        high_frequency_carryover_target: Some(0),
        root_affix_target: Some(root_affix),
        root_affix_base_target: Some(root_affix),
        root_affix_carryover_target: Some(0),
    }
}

fn align_today_targets_to_available_pools(
    _conn: &word_storage_core::Connection,
    targets: TodayTargetSeed,
    _plan_value: &serde_json::Value,
) -> Result<TodayTargetSeed, String> {
    // Today displays the plan target. Availability shortfalls belong to the
    // Study generation/repair path; silently shrinking Today reintroduces
    // Plan/Today/Study denominator drift such as 28 planned review questions
    // rendering as 25 on the Today page.
    Ok(targets)
}

fn plan_unit_count(plan_value: &serde_json::Value, key: &str) -> usize {
    plan_value
        .get(key)
        .and_then(|value| value.as_i64())
        .unwrap_or(0)
        .max(0) as usize
}

fn new_word_question_count_from_plan(
    plan_value: &serde_json::Value,
    key: &str,
    today_date: &str,
) -> u32 {
    let raw = grown_plan_unit_count(plan_value, "newWord", key, today_date);
    raw - (raw % 4)
}

fn new_word_word_count_from_questions(question_count: u32) -> u32 {
    question_count / 4
}

fn grown_plan_unit_count(
    plan_value: &serde_json::Value,
    mode: &str,
    plan_key: &str,
    today_date: &str,
) -> u32 {
    let base = plan_unit_count(plan_value, plan_key) as u32;
    if !plan_value
        .get("growthRuleEnabled")
        .and_then(|value| value.as_bool())
        .unwrap_or(true)
    {
        return base;
    }
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

fn question_type_weights_by_mode_from_value(value: &serde_json::Value) -> serde_json::Value {
    normalize_question_type_weights(
        value
            .get("questionTypeWeightsByMode")
            .unwrap_or(&serde_json::Value::Null),
    )
}

fn normalize_question_type_weights_on_plan(plan: &mut serde_json::Value) {
    let normalized = question_type_weights_by_mode_from_value(plan);
    if let Some(object) = plan.as_object_mut() {
        object.insert("questionTypeWeightsByMode".to_string(), normalized);
    }
}

fn normalize_question_type_weights(value: &serde_json::Value) -> serde_json::Value {
    let existing = value.as_object();
    let mut out = serde_json::Map::new();
    for (mode, allowed) in default_question_type_weights_by_mode() {
        let mode_value = existing
            .and_then(|modes| modes.get(mode))
            .unwrap_or(&serde_json::Value::Null);
        out.insert(
            mode.to_string(),
            normalize_mode_question_type_weights(mode_value, allowed),
        );
    }
    serde_json::Value::Object(out)
}

fn normalize_mode_question_type_weights(
    value: &serde_json::Value,
    defaults: &[(&'static str, i64)],
) -> serde_json::Value {
    let mut raw = serde_json::Map::new();
    let mut total = 0i64;
    let object = value.as_object();
    for (question_type, default_weight) in defaults {
        let weight = object
            .and_then(|weights| weights.get(*question_type))
            .and_then(json_i64_value)
            .unwrap_or(*default_weight)
            .clamp(0, 100);
        raw.insert(
            (*question_type).to_string(),
            serde_json::Value::from(weight),
        );
        total = total.saturating_add(weight);
    }

    if total <= 0 {
        raw.clear();
        total = 0;
        for (question_type, default_weight) in defaults {
            raw.insert(
                (*question_type).to_string(),
                serde_json::Value::from(*default_weight),
            );
            total = total.saturating_add(*default_weight);
        }
    }

    let mut normalized = serde_json::Map::new();
    let mut rounded_total = 0i64;
    let mut first_key: Option<String> = None;
    for (question_type, _) in defaults {
        let key = (*question_type).to_string();
        if first_key.is_none() {
            first_key = Some(key.clone());
        }
        let weight = raw.get(&key).and_then(|value| value.as_i64()).unwrap_or(0);
        let rounded = ((weight as f64 / total as f64) * 100.0).round() as i64;
        rounded_total = rounded_total.saturating_add(rounded);
        normalized.insert(key, serde_json::Value::from(rounded));
    }
    if let Some(key) = first_key {
        let corrected = normalized
            .get(&key)
            .and_then(|value| value.as_i64())
            .unwrap_or(0)
            .saturating_add(100 - rounded_total);
        normalized.insert(key, serde_json::Value::from(corrected));
    }

    serde_json::Value::Object(normalized)
}

fn default_question_type_weights_by_mode() -> [(&'static str, &'static [(&'static str, i64)]); 4] {
    [
        (
            "newWord",
            &[
                ("enToCnChoice", 30),
                ("exampleToCnChoice", 25),
                ("exampleToCnChoiceNoTranslation", 15),
                ("cnToEnChoice", 15),
                ("wordSkeletonInput", 15),
            ],
        ),
        (
            "review",
            &[
                ("enToCnInput", 25),
                ("exampleToCnChoiceNoTranslation", 25),
                ("enToCnChoice", 20),
                ("cnToEnChoice", 15),
                ("wordSkeletonInput", 15),
            ],
        ),
        (
            "mixedTest",
            &[
                ("enToCnChoice", 25),
                ("exampleToCnChoice", 15),
                ("exampleToCnChoiceNoTranslation", 20),
                ("enToCnInput", 20),
                ("cnToEnChoice", 10),
                ("wordSkeletonInput", 10),
            ],
        ),
        (
            "wrongWordReinforcement",
            &[
                ("enToCnInput", 30),
                ("wordSkeletonInput", 25),
                ("exampleToCnChoiceNoTranslation", 20),
                ("cnToEnChoice", 15),
                ("enToCnChoice", 10),
            ],
        ),
    ]
}

fn question_type_weights_for_session_mode(
    conn: &word_storage_core::Connection,
    mode: &SessionMode,
) -> Result<Vec<QuestionTypeWeight>, String> {
    let plan = get_json_setting(conn, "today_plan_json", &default_plan_json())?;
    let normalized = question_type_weights_by_mode_from_value(&plan);
    let mode_key = session_mode_key(mode);
    if mode_key.is_empty() {
        return Ok(Vec::new());
    }
    let weights = normalized
        .get(mode_key)
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for (question_type, weight) in weights {
        let Some(weight) = json_i64_value(&weight) else {
            continue;
        };
        if weight <= 0 {
            continue;
        }
        let question_type = serde_json::from_value::<word_storage_core::models::QuestionType>(
            serde_json::Value::String(question_type),
        )
        .map_err(|e| format!("Invalid question type weight for {mode_key}: {e}"))?;
        out.push(QuestionTypeWeight {
            question_type,
            weight: weight as u32,
        });
    }
    Ok(out)
}

fn session_mode_key(mode: &SessionMode) -> &'static str {
    match mode {
        SessionMode::NewWord => "newWord",
        SessionMode::Review => "review",
        SessionMode::MixedTest => "mixedTest",
        SessionMode::WrongWordReinforcement => "wrongWordReinforcement",
        SessionMode::HighFrequency => "",
        SessionMode::RootAffix => "",
    }
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
    if !next_obj.contains_key("growthRuleEnabled") {
        next_obj.insert(
            "growthRuleEnabled".to_string(),
            previous_plan
                .get("growthRuleEnabled")
                .cloned()
                .unwrap_or(serde_json::Value::Bool(true)),
        );
    }

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
        "highFrequencyPerDay": 10,
        "rootAffixPerDay": 2,
        "growthRuleEnabled": true,
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
            "highFrequency": shared_growth_rule.clone(),
            "rootAffix": shared_growth_rule.clone()
        },
        "questionTypeWeightsByMode": {
            "newWord": {
                "enToCnChoice": 30,
                "exampleToCnChoice": 25,
                "exampleToCnChoiceNoTranslation": 15,
                "cnToEnChoice": 15,
                "wordSkeletonInput": 15
            },
            "review": {
                "enToCnInput": 25,
                "exampleToCnChoiceNoTranslation": 25,
                "enToCnChoice": 20,
                "cnToEnChoice": 15,
                "wordSkeletonInput": 15
            },
            "mixedTest": {
                "enToCnChoice": 25,
                "exampleToCnChoice": 15,
                "exampleToCnChoiceNoTranslation": 20,
                "enToCnInput": 20,
                "cnToEnChoice": 10,
                "wordSkeletonInput": 10
            },
            "wrongWordReinforcement": {
                "enToCnInput": 30,
                "wordSkeletonInput": 25,
                "exampleToCnChoiceNoTranslation": 20,
                "cnToEnChoice": 15,
                "enToCnChoice": 10
            }
        },
        "growthRuleStartDatesByMode": {
            "newWord": today,
            "review": today,
            "mixedTest": today,
            "wrongWordReinforcement": today,
            "highFrequency": today,
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
            status_obj.insert(
                "localCloudRestoreDiagnostics".to_string(),
                build_local_cloud_restore_diagnostics(conn)?,
            );
        }
        serde_json::to_string(&status).map_err(|e| format!("JSON serialization failed: {}", e))
    })
}

fn count_query(conn: &word_storage_core::Connection, sql: &str) -> Result<i64, String> {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0))
        .map(|count| count.max(0))
        .map_err(|e| format!("Failed to run diagnostic count `{sql}`: {e}"))
}

fn build_local_cloud_restore_diagnostics(
    conn: &word_storage_core::Connection,
) -> Result<serde_json::Value, String> {
    let ai_history = get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
    let ai_passage_count = ai_history.as_array().map(Vec::len).unwrap_or(0);
    let ai_passages_with_word_segments = ai_history
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter(|passage| ai_passage_has_word_segments(passage))
                .count()
        })
        .unwrap_or(0);
    let wrong_word_visible_count = load_wrong_word_entries(conn)?.len();
    let reports_history_count = load_reports_history(conn)?.len();
    Ok(serde_json::json!({
        "studyResultsCount": count_query(conn, "SELECT COUNT(*) FROM study_results")?,
        "cloudRestoreStudyResultsCount": count_query(conn, "SELECT COUNT(*) FROM study_results WHERE question_id LIKE 'cloud_restore:%'")?,
        "cloudRestoreSessionsCount": count_query(conn, "SELECT COUNT(*) FROM study_sessions WHERE session_id LIKE 'cloud_restore:%'")?,
        "wrongWordVisibleCount": wrong_word_visible_count,
        "reportsHistoryCount": reports_history_count,
        "aiPassageCount": ai_passage_count,
        "aiPassagesWithWordSegments": ai_passages_with_word_segments,
    }))
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
            "wrongWordRows": request.get("wrongWordRows").and_then(json_i64_value).unwrap_or(0),
            "aiPassageRows": request.get("aiPassageRows").and_then(json_i64_value).unwrap_or(0),
            "restoredStudyPoints": request.get("restoredStudyPoints").and_then(json_i64_value).unwrap_or(0),
            "restoredReportSnapshots": request.get("restoredReportSnapshots").and_then(json_i64_value).unwrap_or(0),
            "restoredWordHints": request.get("restoredWordHints").and_then(json_i64_value).unwrap_or(0),
            "restoredAiPassages": request.get("restoredAiPassages").and_then(json_i64_value).unwrap_or(0),
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
        let study_events = match build_all_study_events_payload(conn)? {
            Some(payload) => {
                let event_count = payload
                    .get("events")
                    .and_then(|value| value.as_array())
                    .map(|items| items.len())
                    .unwrap_or(0);
                enqueue_sync_snapshot(
                    conn,
                    "study_events",
                    &payload,
                    "study_events:all_local_results",
                );
                event_count
            }
            None => 0,
        };
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
            "enqueued": study_events > 0 || study_points > 0 || wrong_words > 0 || ai_passages > 0,
            "studyEvents": study_events,
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
    "high_frequency_pool_json",
    "today_reward_state_json",
    "ai_passage_history_json",
    "cloud_report_snapshots_json",
    "croc_bti_profile_json",
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
            'cloud_report_snapshots_json',
            'croc_bti_profile_json'
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
        ensure_seed_vocabulary_imported(conn, &runtime.paths().bundled_resource_path(""))?;
        if request
            .get("mergeStudyEventsOnly")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
        {
            let merged = merge_cloud_study_events(conn, request.get("studyEvents"))?;
            return serde_json::to_string(
                &serde_json::json!({ "restored": false, "mergedStudyEvents": merged }),
            )
            .map_err(|e| format!("JSON serialization failed: {e}"));
        }
        clear_cloud_restored_learning(conn)?;
        let restored_plan = restore_cloud_plan_config(conn, request.get("planConfig"))?;
        let restored_wordbooks =
            restore_cloud_wordbook_preferences(conn, request.get("wordbookPreferences"))?;
        let restored_points = restore_cloud_study_word_points(
            conn,
            request.get("studyWordPoints"),
            request.get("wrongWordEntries"),
        )?;
        let restored_report_snapshots =
            restore_cloud_report_snapshots(conn, request.get("reportSnapshots"))?;
        let restored_word_hints =
            restore_cloud_wrong_word_hints(conn, request.get("wrongWordEntries"))?;
        let restored_ai_passages = restore_cloud_ai_passages(conn, request.get("aiPassages"))?;
        let restored_croc_bti_profile =
            restore_cloud_croc_bti_profile(conn, request.get("crocBtiProfile"))?;
        ensure_planning_state(conn)?;
        save_local_account_snapshot(conn, user_id)?;
        serde_json::to_string(&serde_json::json!({
            "restored": true,
            "restoredPlan": restored_plan,
            "restoredWordbooks": restored_wordbooks,
            "restoredStudyPoints": restored_points,
            "restoredReportSnapshots": restored_report_snapshots,
            "restoredWordHints": restored_word_hints,
            "restoredAiPassages": restored_ai_passages,
            "restoredCrocBtiProfile": restored_croc_bti_profile,
        }))
        .map_err(|e| format!("JSON serialization failed: {e}"))
    })
}

fn merge_cloud_study_events(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<usize, String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(0);
    };
    let mut merged = 0usize;
    for item in items {
        if item.get("event_type").and_then(|value| value.as_str()) != Some("answer_submitted") {
            continue;
        }
        let Some(payload) = item.get("payload_json").and_then(|value| value.as_object()) else {
            continue;
        };
        let event_id = payload
            .get("eventId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let question_id = payload
            .get("questionId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let entry_id = payload
            .get("entryId")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);
        let entry_source_id = payload
            .get("entrySourceId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let book_id = payload
            .get("bookId")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let book_version = payload
            .get("bookVersion")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let mode = payload
            .get("mode")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let question_type = payload
            .get("questionType")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let outcome = payload
            .get("outcome")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let user_response = payload
            .get("userResponse")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let canonical_answer = payload
            .get("canonicalAnswer")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        let response_time_ms = payload
            .get("responseTimeMs")
            .and_then(|value| value.as_i64())
            .unwrap_or(-1);
        let hint_used = payload.get("hintUsed").and_then(|value| value.as_bool());
        let answered_at = item
            .get("occurred_at")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim();
        if payload
            .get("schemaVersion")
            .and_then(|value| value.as_i64())
            != Some(1)
            || event_id.is_empty()
            || question_id.is_empty()
            || entry_id <= 0
            || entry_source_id.is_empty()
            || book_id.is_empty()
            || book_version.is_empty()
            || !matches!(
                mode,
                "newWord"
                    | "review"
                    | "mixedTest"
                    | "wrongWordReinforcement"
                    | "highFrequency"
                    | "rootAffix"
            )
            || !matches!(
                question_type,
                "enToCnChoice"
                    | "exampleToCnChoice"
                    | "exampleToCnChoiceNoTranslation"
                    | "exampleComprehensionChoice"
                    | "cnToEnChoice"
                    | "enToCnInput"
                    | "wordSkeletonInput"
                    | "glossToRootInput"
                    | "rootToGlossInput"
            )
            || !matches!(
                outcome,
                "correct" | "fuzzyCorrect" | "incorrect" | "skipped"
            )
            || canonical_answer.is_empty()
            || response_time_ms < 0
            || hint_used.is_none()
            || answered_at.is_empty()
        {
            continue;
        }
        let entry_matches = conn
            .query_row(
                "SELECT EXISTS(
                   SELECT 1
                   FROM entries e
                   JOIN wordbook_entries we ON we.entry_id = e.id
                   JOIN wordbooks wb ON wb.id = we.wordbook_id
                   WHERE e.id = ?1 AND e.source_entry_key = ?2 AND wb.code = ?3
                 )",
                rusqlite::params![entry_id, entry_source_id, book_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value != 0)
            .map_err(|e| format!("Failed to validate shared study entry: {e}"))?;
        if !entry_matches {
            continue;
        }
        let marker_key = format!("cloud_study_event:{event_id}");
        let transaction = conn
            .unchecked_transaction()
            .map_err(|e| format!("Failed to start shared study event merge: {e}"))?;
        let already_merged = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM app_settings WHERE key = ?1)",
                [marker_key.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map(|value| value != 0)
            .map_err(|e| format!("Failed to check shared study event marker: {e}"))?;
        if already_merged {
            transaction
                .rollback()
                .map_err(|e| format!("Failed to close duplicate merge: {e}"))?;
            continue;
        }
        let session_id = format!(
            "cloud_event:{}",
            item.get("session_id")
                .and_then(|value| value.as_str())
                .unwrap_or("shared")
        );
        transaction.execute("INSERT OR IGNORE INTO study_sessions (session_id, mode, total_words, wordbook_id, started_at, completed_at) VALUES (?1, ?2, 0, NULL, ?3, ?3)", rusqlite::params![session_id, mode, answered_at])
            .map_err(|e| format!("Failed to create shared study session: {e}"))?;
        transaction.execute("INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response, normalized_response, correct_answer, outcome, response_time_ms, answered_at, hint_used) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?10)", rusqlite::params![session_id, format!("cloud_event:{event_id}:{question_id}"), entry_id, question_type, user_response, canonical_answer, outcome, response_time_ms, answered_at, i64::from(hint_used.unwrap_or(false))])
            .map_err(|e| format!("Failed to insert shared study result: {e}"))?;
        transaction.execute("UPDATE study_sessions SET total_words = (SELECT COUNT(*) FROM study_results WHERE session_id = ?1) WHERE session_id = ?1", [session_id.as_str()])
            .map_err(|e| format!("Failed to update shared study session count: {e}"))?;
        transaction.execute("INSERT INTO app_settings (key, value_json, updated_at) VALUES (?1, '{\"merged\":true}', datetime('now'))", [marker_key.as_str()])
            .map_err(|e| format!("Failed to mark shared study event: {e}"))?;
        transaction
            .commit()
            .map_err(|e| format!("Failed to commit shared study event: {e}"))?;
        merged = merged.saturating_add(1);
    }
    Ok(merged)
}

pub fn restore_cloud_ai_passage_snapshot(request_json: String) -> Result<String, String> {
    let request: serde_json::Value =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    with_runtime_conn(|conn| {
        let restored = restore_cloud_ai_passages(conn, request.get("aiPassages"))?;
        serde_json::to_string(&serde_json::json!({
            "restored": true,
            "restoredAiPassages": restored,
        }))
        .map_err(|e| format!("JSON serialization failed: {e}"))
    })
}

fn clear_cloud_restored_learning(conn: &word_storage_core::Connection) -> Result<(), String> {
    conn.execute(
        "DELETE FROM study_results WHERE question_id LIKE 'cloud_restore:%'",
        [],
    )
    .map_err(|e| format!("Failed to clear restored cloud study results: {e}"))?;
    conn.execute(
        "DELETE FROM study_sessions WHERE session_id LIKE 'cloud_restore:%'",
        [],
    )
    .map_err(|e| format!("Failed to clear restored cloud study sessions: {e}"))?;
    Ok(())
}

fn restore_cloud_croc_bti_profile(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<bool, String> {
    let Some(profile) = value else {
        return Ok(false);
    };
    if !profile.is_object() {
        return Ok(false);
    }
    let result_code = profile
        .get("result_code")
        .or_else(|| profile.get("resultCode"))
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim();
    if result_code.len() != 4 {
        return Ok(false);
    }
    let normalized = serde_json::json!({
        "resultCode": result_code,
        "title": profile.get("title").and_then(|value| value.as_str()).unwrap_or(""),
        "summary": profile.get("summary").and_then(|value| value.as_str()).unwrap_or(""),
        "advice": profile.get("advice").and_then(|value| value.as_str()).unwrap_or(""),
        "answers": profile.get("answers_json").or_else(|| profile.get("answers")).cloned().unwrap_or_else(|| serde_json::json!({})),
        "axisScores": profile.get("axis_scores_json").or_else(|| profile.get("axisScores")).cloned().unwrap_or_else(|| serde_json::json!({})),
        "weights": profile.get("weights_json").or_else(|| profile.get("weights")).cloned().unwrap_or_else(|| serde_json::json!({})),
        "planInput": profile.get("plan_input_json").or_else(|| profile.get("planInput")).cloned().unwrap_or_else(|| serde_json::json!({})),
        "questionTypeWeightsByMode": profile.get("question_type_weights_json").or_else(|| profile.get("questionTypeWeightsByMode")).cloned().unwrap_or_else(|| serde_json::json!({})),
        "dailyLearningMinutes": profile.get("daily_learning_minutes").or_else(|| profile.get("dailyLearningMinutes")).and_then(|value| value.as_i64()).unwrap_or(40),
        "source": profile.get("source").and_then(|value| value.as_str()).unwrap_or("croc_bti"),
        "version": profile.get("version").and_then(|value| value.as_i64()).unwrap_or(1),
        "evaluatedAt": profile.get("evaluated_at").or_else(|| profile.get("evaluatedAt")).and_then(|value| value.as_str()).unwrap_or(""),
    });
    set_json_setting(conn, "croc_bti_profile_json", &normalized)?;
    Ok(true)
}

fn restore_cloud_ai_passages(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
) -> Result<usize, String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(0);
    };
    let mut history = get_json_setting(conn, "ai_passage_history_json", &serde_json::json!([]))?;
    let history_array = history
        .as_array_mut()
        .ok_or("ai_passage_history_json is not an array")?;
    let mut restored = 0usize;
    for item in items {
        let mut passage = decode_cloud_payload_object(item, &["payload_json", "payloadJson"])
            .unwrap_or_else(|| item.clone());
        attach_cloud_ai_wrong_words(conn, &mut passage)?;
        if let Some(object) = passage.as_object_mut() {
            if object
                .get("passageId")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .is_empty()
            {
                if let Some(passage_id) = item
                    .get("passage_id")
                    .or_else(|| item.get("passageId"))
                    .and_then(|value| value.as_str())
                {
                    object.insert(
                        "passageId".to_string(),
                        serde_json::Value::String(passage_id.to_string()),
                    );
                }
            }
            if object
                .get("title")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .is_empty()
            {
                if let Some(title) = item.get("title").and_then(|value| value.as_str()) {
                    object.insert(
                        "title".to_string(),
                        serde_json::Value::String(title.to_string()),
                    );
                }
            }
            if object.get("validationStatus").is_none() {
                if let Some(status) = item
                    .get("validation_status")
                    .or_else(|| item.get("validationStatus"))
                    .and_then(|value| value.as_str())
                {
                    object.insert(
                        "validationStatus".to_string(),
                        serde_json::Value::String(status.to_string()),
                    );
                }
            }
            if object.get("generatedAt").is_none() {
                if let Some(generated_at) = item
                    .get("generated_at")
                    .or_else(|| item.get("generatedAt"))
                    .and_then(|value| value.as_str())
                {
                    object.insert(
                        "generatedAt".to_string(),
                        serde_json::Value::String(generated_at.to_string()),
                    );
                }
            }
        }
        let Some(passage_id) = passage
            .get("passageId")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
        else {
            continue;
        };
        normalize_restored_ai_passage_blocks(&mut passage);
        clean_seed_meaning_noise_in_json(&mut passage);
        history_array.retain(|existing| {
            existing
                .get("passageId")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                != passage_id
        });
        history_array.insert(0, passage);
        restored = restored.saturating_add(1);
    }
    clean_seed_meaning_noise_in_json(&mut history);
    set_json_setting(conn, "ai_passage_history_json", &history)?;
    Ok(restored)
}

fn decode_cloud_payload_object(
    item: &serde_json::Value,
    payload_keys: &[&str],
) -> Option<serde_json::Value> {
    for key in payload_keys {
        if let Some(decoded) = item.get(*key).and_then(decode_jsonish_value) {
            return Some(decoded);
        }
    }
    None
}

fn decode_jsonish_value(value: &serde_json::Value) -> Option<serde_json::Value> {
    if let Some(text) = value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return serde_json::from_str::<serde_json::Value>(text).ok();
    }
    if value.is_object() || value.is_array() {
        return Some(value.clone());
    }
    None
}

fn attach_cloud_ai_wrong_words(
    conn: &word_storage_core::Connection,
    passage: &mut serde_json::Value,
) -> Result<(), String> {
    if passage
        .get("wrongWords")
        .and_then(|value| value.as_array())
        .is_some_and(|items| !items.is_empty())
    {
        return Ok(());
    }
    let Some(covered_ids) = passage
        .get("coveredWordIds")
        .or_else(|| passage.get("covered_word_ids"))
        .and_then(|value| value.as_array())
    else {
        return Ok(());
    };
    let mut wrong_words = Vec::new();
    for entry_id in covered_ids.iter().filter_map(json_i64_value) {
        if entry_id <= 0 {
            continue;
        }
        if let Some(metadata) = ai_passage_wrong_word_metadata_for_entry(conn, entry_id)? {
            wrong_words.push(metadata);
        }
    }
    if wrong_words.is_empty() {
        return Ok(());
    }
    if let Some(object) = passage.as_object_mut() {
        object.insert(
            "wrongWords".to_string(),
            serde_json::Value::Array(wrong_words),
        );
    }
    Ok(())
}

fn ai_passage_wrong_word_metadata_for_entry(
    conn: &word_storage_core::Connection,
    entry_id: i64,
) -> Result<Option<serde_json::Value>, String> {
    let row = conn
        .query_row(
            "SELECT e.word,
                    COALESCE(e.part_of_speech, ''),
                    COALESCE((
                        SELECT meaning_cn
                        FROM entry_meanings
                        WHERE entry_id = e.id
                        ORDER BY id ASC
                        LIMIT 1
                    ), '')
             FROM entries e
             WHERE e.id = ?1",
            [entry_id],
            |row| {
                Ok(serde_json::json!({
                    "entryId": entry_id,
                    "word": row.get::<_, String>(0)?,
                    "partOfSpeech": row.get::<_, String>(1)?,
                    "primaryGloss": row.get::<_, String>(2)?,
                }))
            },
        )
        .optional()
        .map_err(|e| format!("Failed to load AI passage entry metadata {entry_id}: {e}"))?;
    Ok(row)
}

fn normalize_restored_ai_passage_blocks(passage: &mut serde_json::Value) {
    if ai_passage_has_word_segments(passage) {
        return;
    }
    let wrong_words = ai_passage_wrong_word_lookup(passage);
    if wrong_words.is_empty() {
        return;
    }
    let Some(blocks) = passage
        .get_mut("blocks")
        .and_then(|value| value.as_array_mut())
    else {
        return;
    };
    for block in blocks {
        if block
            .get("segments")
            .and_then(|value| value.as_array())
            .is_some()
        {
            continue;
        }
        let text = block
            .as_str()
            .map(str::to_string)
            .or_else(|| {
                block
                    .get("text")
                    .or_else(|| block.get("content"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_default();
        if text.trim().is_empty() {
            continue;
        }
        let block_type = block
            .get("blockType")
            .and_then(|value| value.as_str())
            .unwrap_or("paragraph")
            .to_string();
        *block = serde_json::json!({
            "blockType": block_type,
            "segments": highlight_ai_passage_text_segments(&text, &wrong_words),
        });
    }
}

fn ai_passage_has_word_segments(passage: &serde_json::Value) -> bool {
    passage
        .get("blocks")
        .and_then(|value| value.as_array())
        .map(|blocks| {
            blocks.iter().any(|block| {
                block
                    .get("segments")
                    .and_then(|value| value.as_array())
                    .map(|segments| {
                        segments.iter().any(|segment| {
                            segment
                                .get("type")
                                .and_then(|value| value.as_str())
                                .unwrap_or("")
                                == "word"
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn ai_passage_wrong_word_lookup(
    passage: &serde_json::Value,
) -> BTreeMap<String, serde_json::Value> {
    let mut lookup = BTreeMap::<String, serde_json::Value>::new();
    if let Some(items) = passage.get("wrongWords").and_then(|value| value.as_array()) {
        for item in items {
            let word = item
                .get("word")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim();
            if word.is_empty() {
                continue;
            }
            lookup.insert(word.to_ascii_lowercase(), item.clone());
        }
    }
    lookup
}

fn highlight_ai_passage_text_segments(
    text: &str,
    wrong_words: &BTreeMap<String, serde_json::Value>,
) -> Vec<serde_json::Value> {
    let lower = text.to_ascii_lowercase();
    let mut cursor = 0usize;
    let mut segments = Vec::new();
    while cursor < text.len() {
        let mut best: Option<(usize, usize, &serde_json::Value)> = None;
        for (word, metadata) in wrong_words {
            if word.is_empty() {
                continue;
            }
            if let Some(relative) = lower[cursor..].find(word) {
                let start = cursor + relative;
                let end = start + word.len();
                let is_better = best
                    .map(|(best_start, best_end, _)| {
                        start < best_start || (start == best_start && end > best_end)
                    })
                    .unwrap_or(true);
                if is_better {
                    best = Some((start, end, metadata));
                }
            }
        }
        let Some((start, end, metadata)) = best else {
            segments.push(serde_json::json!({
                "type": "text",
                "text": &text[cursor..],
            }));
            break;
        };
        if start > cursor {
            segments.push(serde_json::json!({
                "type": "text",
                "text": &text[cursor..start],
            }));
        }
        segments.push(serde_json::json!({
            "type": "word",
            "text": &text[start..end],
            "entryId": metadata
                .get("entryId")
                .or_else(|| metadata.get("entry_id"))
                .and_then(json_i64_value)
                .unwrap_or(0),
            "glossZh": metadata
                .get("primaryGloss")
                .or_else(|| metadata.get("glossZh"))
                .and_then(|value| value.as_str())
                .unwrap_or(""),
            "highlighted": true,
        }));
        cursor = end;
    }
    segments
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
        "highFrequencyPerDay",
        plan_row.get("high_frequency_per_day"),
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
    set_json_field(
        plan_obj,
        "questionTypeWeightsByMode",
        plan_row.get("question_type_weights_by_mode"),
    );
    normalize_question_type_weights_on_plan(&mut plan);
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
        .or_else(|| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .and_then(|value| value.parse::<i64>().ok())
        })
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
    let mut merged = get_json_setting(conn, "cloud_report_snapshots_json", &serde_json::json!([]))?;
    let merged_array = merged
        .as_array_mut()
        .ok_or("cloud_report_snapshots_json is not an array")?;
    let mut restored = 0usize;
    for snapshot in snapshots {
        let Some(snapshot_date) = snapshot
            .get("snapshotDate")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
        else {
            continue;
        };
        merged_array.retain(|existing| {
            existing
                .get("snapshotDate")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                != snapshot_date
        });
        merged_array.push(snapshot);
        restored = restored.saturating_add(1);
    }
    merged_array.sort_by(|left, right| {
        left.get("snapshotDate")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .cmp(
                right
                    .get("snapshotDate")
                    .and_then(|value| value.as_str())
                    .unwrap_or(""),
            )
    });
    set_json_setting(conn, "cloud_report_snapshots_json", &merged)?;
    Ok(restored)
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
        let item = decode_cloud_payload_object(item, &["payload_json", "payloadJson", "payload"])
            .unwrap_or_else(|| item.clone());
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
        restored += 1;
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
    }
    Ok(restored)
}

fn restore_cloud_study_word_points(
    conn: &word_storage_core::Connection,
    value: Option<&serde_json::Value>,
    wrong_word_entries: Option<&serde_json::Value>,
) -> Result<usize, String> {
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return Ok(0);
    };
    let mut remaining_wrong_counts = cloud_wrong_word_error_count_map(wrong_word_entries);
    let mut session_totals: BTreeMap<(String, String), i64> = BTreeMap::new();
    for point in items {
        let Some((date, mode, _, _, _, _, _, _, _)) = parse_cloud_study_word_point(point) else {
            continue;
        };
        *session_totals.entry((date, mode)).or_insert(0) += 1;
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
            _attempt_count,
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
        let capped_wrong_count =
            consume_cloud_wrong_count_budget(&mut remaining_wrong_counts, entry_id, wrong_count);
        let outcome = if capped_wrong_count > 0 {
            "incorrect"
        } else if correct_count > 0 {
            "correct"
        } else {
            "skipped"
        };
        insert_cloud_restored_study_result(
            conn,
            &date,
            &mode,
            entry_id,
            &question_type,
            1,
            outcome,
            total_response_time_ms.max(0),
            &last_answered_at,
        )?;
        restored_attempts += 1;
    }
    Ok(restored_attempts)
}

type CloudStudyWordPoint = (String, String, i64, String, i64, i64, i64, i64, String);

fn cloud_wrong_word_error_count_map(value: Option<&serde_json::Value>) -> BTreeMap<i64, i64> {
    let mut result = BTreeMap::new();
    let Some(items) = value.and_then(|value| value.as_array()) else {
        return result;
    };
    for item in items {
        let item = decode_cloud_payload_object(item, &["payload_json", "payloadJson", "payload"])
            .unwrap_or_else(|| item.clone());
        let Some(entry_id) = item
            .get("entry_id")
            .or_else(|| item.get("entryId"))
            .and_then(json_i64_value)
        else {
            continue;
        };
        let error_count = item
            .get("error_count")
            .or_else(|| item.get("errorCount"))
            .and_then(json_i64_value)
            .unwrap_or(0)
            .clamp(0, 20);
        if entry_id > 0 && error_count > 0 {
            result.insert(entry_id, error_count);
        }
    }
    result
}

fn consume_cloud_wrong_count_budget(
    remaining_wrong_counts: &mut BTreeMap<i64, i64>,
    entry_id: i64,
    requested_wrong_count: i64,
) -> i64 {
    let requested_wrong_count = requested_wrong_count.max(0);
    if requested_wrong_count == 0 {
        return 0;
    }
    let Some(remaining) = remaining_wrong_counts.get_mut(&entry_id) else {
        return 0;
    };
    let allowed = requested_wrong_count.min((*remaining).max(0));
    *remaining -= allowed;
    allowed
}

fn parse_cloud_study_word_point(point: &serde_json::Value) -> Option<CloudStudyWordPoint> {
    let point = decode_cloud_payload_object(point, &["payload_json", "payloadJson", "payload"])
        .unwrap_or_else(|| point.clone());
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
        "exampleToCnChoiceNoTranslation" => "exampleToCnChoiceNoTranslation".to_string(),
        "cnToEnChoice" => "cnToEnChoice".to_string(),
        "enToCnInput" => "enToCnInput".to_string(),
        "wordSkeletonInput" => "wordSkeletonInput".to_string(),
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
    let mut request: StoredAiProviderConfig =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {e}"))?;
    if request.anthropic_fallback.is_none() {
        request.anthropic_fallback = default_ai_provider_config().anthropic_fallback;
    }
    if request.agnes_fallback.is_none() {
        request.agnes_fallback = default_ai_provider_config().agnes_fallback;
    }
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
    AcceptDisputedMeaningRequest, QuestionTypeWeight, SessionMode, StartSessionEntryPayload,
    StartSessionMeaningPayload, StartSessionRequest, SubmitAnswerRequest,
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

    if let Some(response) = try_resume_empty_start_request(&conn, &request)? {
        let mut payload = serde_json::to_value(&response)
            .map_err(|e| format!("JSON serialization failed: {e}"))?;
        enrich_study_response_hints(&conn, &mut payload)?;
        enrich_mastered_entry_count(&conn, &mut payload)?;
        return clean_json_string(&payload);
    }

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
    enrich_mastered_entry_count(&conn, &mut payload)?;
    clean_json_string(&payload)
}

fn try_resume_empty_start_request(
    conn: &word_storage_core::Connection,
    request: &StartSessionRequest,
) -> Result<Option<word_storage_core::models::StartSessionResponse>, String> {
    if !request.entry_source_ids.is_empty() || !request.entry_payloads.is_empty() {
        return Ok(None);
    }
    let has_snapshot_for_requested_mode =
        persistence::study_repo::load_active_session_snapshot(conn, &request.mode)
            .map_err(|e| format!("Failed to load active session snapshot: {e}"))?
            .is_some();
    if !has_snapshot_for_requested_mode {
        return Ok(None);
    }
    match core_start_study_session(conn, request.clone()) {
        Ok(response) => Ok(Some(response)),
        Err(error) if error.to_string() == "Not enough words" => Ok(None),
        Err(error) => Err(format!("Failed to resume session: {}", error)),
    }
}

fn required_choice_distractor_kinds(
    mode: &SessionMode,
    question_type_weights: &[QuestionTypeWeight],
) -> (bool, bool) {
    use word_storage_core::models::QuestionType;

    if matches!(mode, SessionMode::RootAffix) {
        return (false, false);
    }
    if matches!(mode, SessionMode::NewWord) {
        return (true, true);
    }

    let mut needs_cn = false;
    let mut needs_en = false;
    for weight in question_type_weights
        .iter()
        .filter(|weight| weight.weight > 0)
    {
        match weight.question_type {
            QuestionType::EnToCnChoice
            | QuestionType::ExampleToCnChoice
            | QuestionType::ExampleToCnChoiceNoTranslation => needs_cn = true,
            QuestionType::CnToEnChoice => needs_en = true,
            _ => {}
        }
    }

    if !needs_cn && !needs_en && !question_type_weights.is_empty() {
        return (false, false);
    }

    if !needs_cn && !needs_en {
        match mode {
            SessionMode::Review | SessionMode::MixedTest | SessionMode::WrongWordReinforcement => {
                (true, true)
            }
            SessionMode::NewWord => (true, true),
            SessionMode::HighFrequency => (false, false),
            SessionMode::RootAffix => (false, false),
        }
    } else {
        (needs_cn, needs_en)
    }
}

fn payloads_have_required_precomputed_distractors(
    payloads: &[StartSessionEntryPayload],
    needs_cn: bool,
    needs_en: bool,
) -> bool {
    payloads.iter().all(|payload| {
        (!needs_cn || payload.cn_choice_distractors.len() >= 3)
            && (!needs_en || payload.en_choice_distractors.len() >= 3)
    })
}

fn study_entry_candidate_limit(target_count: usize) -> usize {
    target_count
        .saturating_mul(2)
        .max(target_count.saturating_add(32))
}

fn has_bridge_display_content(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty() && trimmed != "/"
}

fn has_bridge_study_word_content(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains('.') || trimmed == "/" {
        return false;
    }
    trimmed.chars().filter(|ch| ch.is_alphabetic()).count() >= 2
}

fn bridge_payload_meanings(payload: &StartSessionEntryPayload) -> Vec<String> {
    if payload.meaning_details.is_empty() {
        payload.meanings.clone()
    } else {
        payload
            .meaning_details
            .iter()
            .map(|meaning| meaning.meaning_cn.clone())
            .collect()
    }
}

fn is_valid_bridge_study_payload(payload: &StartSessionEntryPayload) -> bool {
    has_bridge_display_content(&payload.source_id)
        && has_bridge_study_word_content(&payload.word)
        && bridge_payload_meanings(payload)
            .iter()
            .any(|meaning| has_bridge_display_content(meaning))
}

fn load_valid_study_payloads_for_candidates(
    conn: &word_storage_core::Connection,
    candidate_entry_ids: &[i64],
    target_count: usize,
) -> Result<(Vec<i64>, Vec<StartSessionEntryPayload>), String> {
    let payloads = load_entry_payloads(conn, candidate_entry_ids)?;
    let mut entry_ids = Vec::with_capacity(target_count);
    let mut valid_payloads = Vec::with_capacity(target_count);
    for (entry_id, payload) in candidate_entry_ids.iter().copied().zip(payloads) {
        if !is_valid_bridge_study_payload(&payload) {
            continue;
        }
        entry_ids.push(entry_id);
        valid_payloads.push(payload);
        if valid_payloads.len() >= target_count {
            break;
        }
    }
    Ok((entry_ids, valid_payloads))
}

fn hydrate_start_session_request(
    conn: &word_storage_core::Connection,
    mut request: StartSessionRequest,
    bundle_resource_dir: Option<std::path::PathBuf>,
) -> Result<StartSessionRequest, String> {
    ensure_planning_state(conn)?;
    if request.question_type_weights.is_empty()
        && session_mode_uses_question_type_weights(&request.mode)
    {
        request.question_type_weights =
            question_type_weights_for_session_mode(conn, &request.mode)?;
    }

    let has_restore_placeholders = request
        .entry_source_ids
        .iter()
        .any(|id| id.starts_with("active_session_restore_"));
    if (!request.entry_source_ids.is_empty() || !request.entry_payloads.is_empty())
        && !has_restore_placeholders
    {
        if !request.entry_source_ids.is_empty() && request.entry_payloads.is_empty() {
            let source_ids = request
                .entry_source_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            let entry_ids = load_entry_ids_for_source_keys(conn, &source_ids)?;
            if entry_ids.len() != source_ids.len() {
                return Err(format!(
                    "Could not hydrate all requested study entries: requested {}, found {}",
                    source_ids.len(),
                    entry_ids.len()
                ));
            }
            request.entry_payloads = load_entry_payloads(conn, &entry_ids)?;
        }
        if request.distractor_payloads.is_empty() && !request.entry_payloads.is_empty() {
            let (needs_cn, needs_en) =
                required_choice_distractor_kinds(&request.mode, &request.question_type_weights);
            if !payloads_have_required_precomputed_distractors(
                &request.entry_payloads,
                needs_cn,
                needs_en,
            ) {
                let excluded_source_ids = request
                    .entry_payloads
                    .iter()
                    .map(|payload| payload.source_id.as_str())
                    .collect::<Vec<_>>();
                request.distractor_payloads =
                    load_distractor_payloads_excluding_sources(conn, &excluded_source_ids, 96)?;
            }
        }
        return Ok(request);
    }
    if has_restore_placeholders {
        request.entry_source_ids.clear();
        request.entry_payloads.clear();
        request.distractor_payloads.clear();
    }

    if let Some(bundle_dir) = bundle_resource_dir.as_deref() {
        ensure_seed_vocabulary_imported(conn, bundle_dir)?;
    }

    let (target_count, planned_target_count) = study_mode_target_counts(conn, &request.mode)?;
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

    let candidate_limit = study_entry_candidate_limit(planned_target_count);
    let mut primary_entry_ids = match request.mode {
        SessionMode::Review => {
            load_review_entry_ids_for_today(conn, &review_wordbook_ids, candidate_limit)?
        }
        SessionMode::MixedTest => load_random_entry_ids_for_wordbooks_with_global_fallback(
            conn,
            &active_wordbook_ids,
            candidate_limit,
        )?,
        SessionMode::WrongWordReinforcement => {
            load_prioritized_wrong_word_entry_ids(conn, candidate_limit)?
        }
        SessionMode::HighFrequency => {
            load_high_frequency_candidate_entry_ids(conn, candidate_limit)?
        }
        SessionMode::NewWord => load_unlearned_ranked_entry_ids_for_wordbooks_with_global_fallback(
            conn,
            &active_wordbook_ids,
            candidate_limit,
        )?,
        SessionMode::RootAffix => unreachable!("root/affix mode handled above"),
    };
    let answered_today =
        load_today_answered_entry_ids_for_mode(conn, &request.mode, &today_date_string())?;
    primary_entry_ids.retain(|entry_id| !answered_today.contains(entry_id));

    let (entry_ids, entry_payloads) =
        load_valid_study_payloads_for_candidates(conn, &primary_entry_ids, target_count)?;
    if entry_payloads.is_empty() {
        return Err("No study entries available from the local vocabulary data".to_string());
    }

    let (needs_cn, needs_en) =
        required_choice_distractor_kinds(&request.mode, &request.question_type_weights);
    let distractor_payloads =
        if payloads_have_required_precomputed_distractors(&entry_payloads, needs_cn, needs_en) {
            Vec::new()
        } else {
            let distractor_target = std::cmp::max(target_count.saturating_mul(12), 96);
            let distractor_ids = load_ranked_entry_ids_for_wordbooks_excluding(
                conn,
                &active_wordbook_ids,
                &entry_ids,
                distractor_target,
            )?;
            load_entry_payloads(conn, &distractor_ids)?
        };

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

fn session_mode_uses_question_type_weights(mode: &SessionMode) -> bool {
    !matches!(
        mode,
        SessionMode::NewWord | SessionMode::HighFrequency | SessionMode::RootAffix
    )
}

fn study_mode_target_counts(
    conn: &word_storage_core::Connection,
    mode: &SessionMode,
) -> Result<(usize, usize), String> {
    let plan = get_json_setting(conn, "today_plan_json", &default_plan_json())?;
    let today = today_date_string();
    let targets = load_today_target_seed(conn, &today)?.unwrap_or_default();
    let completions = load_today_completion_seed(conn, &today)?;
    let (planned_questions, completed_questions) = match mode {
        SessionMode::NewWord => (
            targets.new_words_target.unwrap_or_else(|| {
                new_word_question_count_from_plan(&plan, "newWordsPerDay", &today)
            }),
            completions.new_words_completed,
        ),
        SessionMode::Review => (
            targets.review_words_target.unwrap_or_else(|| {
                grown_plan_unit_count(&plan, "review", "reviewWordsPerDay", &today)
            }),
            completions.review_words_completed,
        ),
        SessionMode::MixedTest => (
            targets.mixed_test_target.unwrap_or_else(|| {
                grown_plan_unit_count(&plan, "mixedTest", "mixedTestPerDay", &today)
            }),
            completions.mixed_test_completed,
        ),
        SessionMode::WrongWordReinforcement => (
            targets.wrong_word_test_target.unwrap_or_else(|| {
                grown_plan_unit_count(
                    &plan,
                    "wrongWordReinforcement",
                    "wrongWordTestPerDay",
                    &today,
                )
            }),
            completions.wrong_word_test_completed,
        ),
        SessionMode::HighFrequency => (
            targets.high_frequency_target.unwrap_or_else(|| {
                grown_plan_unit_count(&plan, "highFrequency", "highFrequencyPerDay", &today)
            }),
            completions.high_frequency_completed,
        ),
        SessionMode::RootAffix => (
            targets.root_affix_target.unwrap_or_else(|| {
                grown_plan_unit_count(&plan, "rootAffix", "rootAffixPerDay", &today)
            }),
            completions.root_affix_completed.unwrap_or(0),
        ),
    };
    let remaining_questions = planned_questions.saturating_sub(completed_questions);
    let planned_entries = if matches!(mode, SessionMode::NewWord) {
        new_word_word_count_from_questions(planned_questions)
    } else {
        planned_questions
    };
    let remaining_entries = if matches!(mode, SessionMode::NewWord) {
        new_word_word_count_from_questions(remaining_questions)
    } else {
        remaining_questions
    };
    Ok((remaining_entries as usize, planned_entries as usize))
}

fn load_today_answered_entry_ids_for_mode(
    conn: &word_storage_core::Connection,
    mode: &SessionMode,
    today_date: &str,
) -> Result<BTreeSet<i64>, String> {
    let expected_mode = serde_json::to_string(mode)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string();
    let mut statement = conn
        .prepare(
            "SELECT s.mode, r.entry_id, r.answered_at
             FROM study_results r
             INNER JOIN study_sessions s ON s.session_id = r.session_id
             WHERE r.entry_id IS NOT NULL",
        )
        .map_err(|error| format!("Failed to prepare today's study entry filter: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| format!("Failed to query today's study entry filter: {error}"))?;
    let mut entry_ids = BTreeSet::new();
    for row in rows {
        let (stored_mode, entry_id, answered_at) =
            row.map_err(|error| format!("Failed to read today's study entry filter: {error}"))?;
        if normalize_stored_session_mode(&stored_mode) == expected_mode
            && local_date_from_rfc3339(&answered_at).as_deref() == Some(today_date)
        {
            entry_ids.insert(entry_id);
        }
    }
    Ok(entry_ids)
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
    let ids = ordered
        .into_iter()
        .take(limit)
        .filter_map(|item| item.get("entryId").and_then(|value| value.as_i64()))
        .filter(|entry_id| *entry_id > 0)
        .collect::<Vec<_>>();
    filter_mastered_entry_ids(conn, ids)
}

fn load_high_frequency_candidate_entry_ids(
    conn: &word_storage_core::Connection,
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mut statement = conn
        .prepare(
            "SELECT DISTINCT e.id
             FROM entries e
             INNER JOIN wordbook_entries we ON we.entry_id = e.id
             INNER JOIN wordbooks wb ON wb.id = we.wordbook_id
             LEFT JOIN mastered_entries me ON me.entry_id = e.id
             WHERE wb.code = 'kaoyan'
               AND e.exam_frequency > 0
               AND e.exam_rank IS NOT NULL
               AND me.entry_id IS NULL
             ORDER BY e.exam_rank ASC, e.id ASC
             LIMIT ?1",
        )
        .map_err(|error| format!("Failed to prepare high-frequency vocabulary: {error}"))?;
    let ranked = statement
        .query_map([limit as i64], |row| row.get::<_, i64>(0))
        .map_err(|error| format!("Failed to query high-frequency vocabulary: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to read high-frequency vocabulary: {error}"))?;

    let candidate_json = serde_json::to_string(&ranked)
        .map_err(|error| format!("Failed to encode high-frequency candidates: {error}"))?;
    let mut random_statement = conn
        .prepare(
            "SELECT CAST(value AS INTEGER)
             FROM json_each(?1)
             ORDER BY RANDOM()
             LIMIT ?2",
        )
        .map_err(|error| format!("Failed to prepare high-frequency draw: {error}"))?;
    let drawn = random_statement
        .query_map(rusqlite::params![candidate_json, limit as i64], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|error| format!("Failed to draw high-frequency vocabulary: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Failed to read high-frequency draw: {error}"))?;
    Ok(drawn)
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
    let all = load_root_affix_cards_cached(bundle_dir)?;
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
    scoped = dedupe_root_affix_cards_for_session(scoped);
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

fn load_root_affix_cards_cached(bundle_dir: &Path) -> Result<Vec<RootAffixCard>, String> {
    let cache_key = bundle_dir.to_string_lossy().to_string();
    if let Some(cards) = ROOT_AFFIX_CARD_CACHE
        .lock()
        .map_err(|_| "Failed to lock root/affix cache".to_string())?
        .get(&cache_key)
        .cloned()
    {
        return Ok(cards);
    }

    let cards = load_root_affix_cards(bundle_dir)?;
    ROOT_AFFIX_CARD_CACHE
        .lock()
        .map_err(|_| "Failed to lock root/affix cache".to_string())?
        .insert(cache_key, cards.clone());
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
        let mut normalized = normalize_root_affix_form(form);
        let mut was_promoted_prefix = false;
        if let Some(promoted) = promote_shared_root_affix_form(&lower_word, &normalized) {
            normalized = promoted.to_string();
            was_promoted_prefix = true;
        }
        if normalized.len() < 2 || normalized.len() >= lower_word.len() || meaning.is_empty() {
            continue;
        }
        if !shared_root_affix_form_matches_word(&lower_word, &normalized, form) {
            continue;
        }
        let display_form = if was_promoted_prefix && lower_word.starts_with(&normalized) {
            format!("{normalized}-")
        } else if form.starts_with('-') || form.ends_with('-') {
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

fn promote_shared_root_affix_form<'a>(word: &str, normalized: &str) -> Option<&'a str> {
    const LONGER_PREFIXES: &[&str] = &[
        "anti", "ante", "auto", "circum", "contra", "extra", "inter", "intra", "intro", "micro",
        "multi", "post", "semi", "super", "trans", "ultra", "abs", "bio", "dia", "dis", "fore",
        "mal", "mis", "non", "pre", "pro", "sub", "sur", "sym", "syn", "tele", "tri", "con", "com",
        "ab", "ad", "de", "di", "ex", "re", "un",
    ];
    LONGER_PREFIXES
        .iter()
        .copied()
        .filter(|candidate| {
            candidate.len() > normalized.len()
                && candidate.starts_with(normalized)
                && word.starts_with(candidate)
        })
        .max_by_key(|candidate| candidate.len())
}

fn shared_root_affix_form_matches_word(word: &str, normalized: &str, raw_form: &str) -> bool {
    if raw_form.starts_with('-') {
        return word.ends_with(normalized);
    }
    if raw_form.ends_with('-') {
        return word.starts_with(normalized);
    }
    word.starts_with(normalized) || word.ends_with(normalized)
}

fn load_medical_root_affix_cards(bundle_dir: &Path) -> Result<Vec<RootAffixCard>, String> {
    let path = bundle_dir
        .join("seed-medical")
        .join("medical-root-affix.txt");
    if !path.exists() {
        return Ok(builtin_medical_root_affix_cards());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read medical root/affix asset: {e}"))?;
    let mut cards: Vec<RootAffixCard> = Vec::new();
    let mut last_index: Option<usize> = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#") {
            continue;
        }
        if let Some(payload) = trimmed.strip_prefix("examples:") {
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
            meaning_cn: sanitize_chinese_meaning(&meaning),
            example_pairs: Vec::new(),
            scope: "medical".to_string(),
        });
        last_index = Some(cards.len() - 1);
    }
    let mut by_id = cards
        .into_iter()
        .filter(is_reliable_root_affix_card)
        .map(|card| (card.id.clone(), card))
        .collect::<BTreeMap<_, _>>();
    for card in builtin_medical_root_affix_cards() {
        by_id.entry(card.id.clone()).or_insert(card);
    }
    Ok(by_id.into_values().collect())
}

fn builtin_medical_root_affix_cards() -> Vec<RootAffixCard> {
    vec![
        medical_root_affix_card(
            "cardi-",
            "\u{5fc3}\u{810f}",
            &[
                ("cardiology", "\u{5fc3}\u{810f}\u{75c5}\u{5b66}"),
                ("cardiopulmonary", "\u{5fc3}\u{80ba}\u{7684}"),
            ],
        ),
        medical_root_affix_card(
            "bronch-",
            "\u{652f}\u{6c14}\u{7ba1}",
            &[
                ("bronchitis", "\u{652f}\u{6c14}\u{7ba1}\u{708e}"),
                (
                    "bronchoscopy",
                    "\u{652f}\u{6c14}\u{7ba1}\u{955c}\u{68c0}\u{67e5}",
                ),
            ],
        ),
        medical_root_affix_card(
            "pneumo-",
            "\u{80ba}",
            &[
                ("pneumonia", "\u{80ba}\u{708e}"),
                ("pneumothorax", "\u{6c14}\u{80f8}"),
            ],
        ),
        medical_root_affix_card(
            "hypo-",
            "\u{4f4e}",
            &[
                ("hypoxia", "\u{7f3a}\u{6c27}"),
                ("hypoxemia", "\u{4f4e}\u{6c27}\u{8840}\u{75c7}"),
            ],
        ),
        medical_root_affix_card(
            "hyper-",
            "\u{9ad8}",
            &[
                ("hypertension", "\u{9ad8}\u{8840}\u{538b}"),
                ("hypercapnia", "\u{9ad8}\u{78b3}\u{9178}\u{8840}\u{75c7}"),
            ],
        ),
        medical_root_affix_card(
            "-itis",
            "\u{708e}\u{75c7}",
            &[
                ("bronchitis", "\u{652f}\u{6c14}\u{7ba1}\u{708e}"),
                ("rhinitis", "\u{9f3b}\u{708e}"),
            ],
        ),
        medical_root_affix_card(
            "-emia",
            "\u{8840}\u{75c7}",
            &[
                ("hypoxemia", "\u{4f4e}\u{6c27}\u{8840}\u{75c7}"),
                ("anemia", "\u{8d2b}\u{8840}"),
            ],
        ),
        medical_root_affix_card(
            "-scopy",
            "\u{955c}\u{68c0}",
            &[
                (
                    "bronchoscopy",
                    "\u{652f}\u{6c14}\u{7ba1}\u{955c}\u{68c0}\u{67e5}",
                ),
                ("endoscopy", "\u{5185}\u{955c}\u{68c0}\u{67e5}"),
            ],
        ),
        medical_root_affix_card(
            "trache-",
            "\u{6c14}\u{7ba1}",
            &[
                ("tracheal", "\u{6c14}\u{7ba1}\u{7684}"),
                ("tracheostomy", "\u{6c14}\u{7ba1}\u{9020}\u{53e3}\u{672f}"),
            ],
        ),
        medical_root_affix_card(
            "pulmon-",
            "\u{80ba}",
            &[
                ("pulmonary", "\u{80ba}\u{7684}"),
                ("extrapulmonary", "\u{80ba}\u{5916}\u{7684}"),
            ],
        ),
    ]
}

fn medical_root_affix_card(
    form: &str,
    meaning_cn: &str,
    example_pairs: &[(&str, &str)],
) -> RootAffixCard {
    let normalized = normalize_root_affix_form(form);
    RootAffixCard {
        id: format!("root_affix_medical_{normalized}"),
        form: form.to_string(),
        meaning_cn: meaning_cn.to_string(),
        example_pairs: example_pairs
            .iter()
            .map(|(word, gloss)| ((*word).to_string(), (*gloss).to_string()))
            .collect(),
        scope: "medical".to_string(),
    }
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
        cn_choice_distractors: Vec::new(),
        en_choice_distractors: Vec::new(),
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

fn dedupe_root_affix_cards_for_session(cards: Vec<RootAffixCard>) -> Vec<RootAffixCard> {
    let mut seen_meanings = BTreeSet::new();
    let mut result = Vec::new();
    for card in cards {
        let key = normalize_root_affix_meaning_key(&card.meaning_cn);
        if key.is_empty() || !seen_meanings.insert(key) {
            continue;
        }
        result.push(card);
    }
    result
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
    for raw in value.split(&[';', ',', '/'][..]) {
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
        parts.join(", ")
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
    if let Some(index) = value.find("->") {
        value[..index].to_string()
    } else {
        value.to_string()
    }
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
        .trim_matches(|ch| matches!(ch, '[' | ']' | ',' | ';' | '/' | ' '))
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

fn normalize_root_affix_meaning_key(value: &str) -> String {
    sanitize_chinese_meaning(value)
        .chars()
        .filter(|ch| !matches!(ch, '\u{ff0c}' | ',' | ';' | '\u{ff1b}' | ' ' | '/'))
        .collect()
}

fn is_reliable_root_affix_card(card: &RootAffixCard) -> bool {
    let normalized = normalize_root_affix_form(&card.form);
    if normalized.len() < 2 || normalized.len() > 6 {
        return false;
    }
    if card.scope == "shared" && !is_reliable_shared_root_affix_form(&normalized, card) {
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

fn is_reliable_shared_root_affix_form(normalized: &str, card: &RootAffixCard) -> bool {
    if !is_allowed_shared_root_affix(normalized, &card.meaning_cn) {
        return false;
    }
    if normalized.len() > 4 || matches!(normalized, "ear" | "exe") {
        return false;
    }
    if normalized
        .chars()
        .all(|ch| matches!(ch, 'a' | 'e' | 'i' | 'o' | 'u'))
    {
        return false;
    }
    if !card.form.ends_with('-') && !card.form.starts_with('-') {
        let prefix_hits = card
            .example_pairs
            .iter()
            .filter(|(word, _)| word.to_lowercase().starts_with(normalized))
            .count();
        let total = card.example_pairs.len().max(1);
        if prefix_hits * 2 < total {
            return false;
        }
    }
    is_concise_root_meaning_clean(&card.meaning_cn)
}

fn is_allowed_shared_root_affix(normalized: &str, meaning: &str) -> bool {
    let meaning_key = normalize_root_affix_meaning_key(meaning);
    const ALLOWED: &[(&str, &[&str])] = &[
        ("ab", &["\u{79bb}\u{5f00}", "\u{8fdc}\u{79bb}"]),
        ("abs", &["\u{79bb}\u{5f00}", "\u{8fdc}\u{79bb}"]),
        ("ad", &["\u{5411}", "\u{671d}\u{5411}", "\u{52a0}\u{5f3a}"]),
        ("ante", &["\u{524d}", "\u{5148}"]),
        ("anti", &["\u{53cd}", "\u{6297}", "\u{76f8}\u{53cd}"]),
        ("auto", &["\u{81ea}\u{5df1}", "\u{81ea}\u{52a8}"]),
        ("bio", &["\u{751f}\u{547d}", "\u{751f}\u{7269}"]),
        ("circ", &["\u{5706}", "\u{73af}"]),
        ("circum", &["\u{5468}\u{56f4}", "\u{73af}\u{7ed5}"]),
        ("com", &["\u{5171}\u{540c}", "\u{4e00}\u{8d77}"]),
        ("con", &["\u{5171}\u{540c}", "\u{4e00}\u{8d77}"]),
        ("contra", &["\u{53cd}\u{5bf9}", "\u{76f8}\u{53cd}"]),
        (
            "de",
            &["\u{5411}\u{4e0b}", "\u{79bb}\u{5f00}", "\u{5426}\u{5b9a}"],
        ),
        ("di", &["\u{4e8c}", "\u{5206}\u{5f00}"]),
        ("dia", &["\u{7a7f}\u{8fc7}", "\u{901a}\u{8fc7}"]),
        ("dis", &["\u{5206}\u{5f00}", "\u{5426}\u{5b9a}", "\u{4e0d}"]),
        ("ex", &["\u{51fa}", "\u{5411}\u{5916}"]),
        ("extra", &["\u{5916}", "\u{8d85}\u{51fa}"]),
        ("fore", &["\u{524d}", "\u{9884}\u{5148}"]),
        ("inter", &["\u{4e4b}\u{95f4}", "\u{76f8}\u{4e92}"]),
        ("intra", &["\u{5185}\u{90e8}", "\u{5185}"]),
        ("intro", &["\u{5411}\u{5185}", "\u{5185}\u{90e8}"]),
        ("mal", &["\u{574f}", "\u{6076}"]),
        ("micro", &["\u{5c0f}", "\u{5fae}"]),
        ("mis", &["\u{9519}\u{8bef}", "\u{574f}"]),
        ("mono", &["\u{5355}", "\u{4e00}"]),
        ("multi", &["\u{591a}"]),
        ("non", &["\u{4e0d}", "\u{65e0}"]),
        ("post", &["\u{540e}"]),
        ("pre", &["\u{524d}", "\u{9884}\u{5148}"]),
        ("pro", &["\u{5411}\u{524d}", "\u{652f}\u{6301}"]),
        ("re", &["\u{518d}", "\u{91cd}\u{65b0}", "\u{56de}"]),
        ("semi", &["\u{534a}"]),
        ("sub", &["\u{4e0b}", "\u{6b21}"]),
        ("super", &["\u{4e0a}", "\u{8d85}\u{8fc7}"]),
        ("sur", &["\u{4e0a}", "\u{8d85}\u{8fc7}"]),
        ("sym", &["\u{5171}\u{540c}", "\u{4e00}\u{8d77}"]),
        ("syn", &["\u{5171}\u{540c}", "\u{4e00}\u{8d77}"]),
        ("tele", &["\u{8fdc}"]),
        (
            "trans",
            &["\u{6a2a}\u{8fc7}", "\u{8f6c}\u{79fb}", "\u{6539}\u{53d8}"],
        ),
        ("tri", &["\u{4e09}"]),
        ("ultra", &["\u{8d85}", "\u{6781}\u{7aef}"]),
        ("un", &["\u{4e0d}", "\u{76f8}\u{53cd}"]),
    ];
    ALLOWED
        .iter()
        .find(|(form, _)| *form == normalized)
        .map(|(_, allowed_meanings)| {
            allowed_meanings.iter().any(|allowed| {
                let allowed_key = normalize_root_affix_meaning_key(allowed);
                meaning_key.contains(&allowed_key) || allowed_key.contains(&meaning_key)
            })
        })
        .unwrap_or(false)
}

fn is_concise_root_meaning_clean(value: &str) -> bool {
    let meaning = sanitize_chinese_meaning(value);
    !meaning.is_empty()
        && meaning.chars().count() <= 8
        && !meaning.contains("\u{7535}\u{8111}")
        && !meaning.contains("\u{6267}\u{884c}\u{6587}\u{4ef6}")
        && !meaning.contains("\u{53ef}\u{6267}\u{884c}")
}

#[allow(dead_code)]
fn is_concise_root_meaning(value: &str) -> bool {
    let meaning = sanitize_chinese_meaning(value);
    !meaning.is_empty()
        && meaning.chars().count() <= 8
        && !meaning.contains("\u{7535}\u{8111}")
        && !meaning.contains("\u{6267}\u{884c}\u{6587}\u{4ef6}")
        && !meaning.contains("\u{53ef}\u{6267}\u{884c}")
}

fn split_medical_examples(payload: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut seen = BTreeSet::new();
    for item in payload.split(&[';', ','][..]) {
        let trimmed = item.trim();
        let parsed = trimmed
            .split_once('(')
            .and_then(|(word, rest)| rest.split_once(')').map(|(gloss, _)| (word, gloss)));
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
    filter_mastered_entry_ids(conn, select_review_candidates(candidates, &today, limit))
}

fn filter_mastered_entry_ids(
    conn: &word_storage_core::Connection,
    entry_ids: Vec<i64>,
) -> Result<Vec<i64>, String> {
    if entry_ids.is_empty() {
        return Ok(entry_ids);
    }
    let mastered = word_storage_core::persistence::mastered_entry_repo::mastered_entry_ids(conn)
        .map_err(|e| format!("Failed to load mastered entries: {e}"))?;
    if mastered.is_empty() {
        return Ok(entry_ids);
    }
    Ok(entry_ids
        .into_iter()
        .filter(|id| !mastered.contains(id))
        .collect())
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
    let mark_priority = exercise_vocab_mark_priority_expression("e.id");
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
         ORDER BY {mark_priority} DESC,
                  ((we.rank_in_book * ?{}) % 9973) ASC,
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
    let ids = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode unlearned wordbook entry seed row: {e}"))?;
    filter_mastered_entry_ids(conn, ids)
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

fn exercise_vocab_mark_priority_expression(entry_id_expression: &str) -> String {
    format!(
        "COALESCE((
            SELECT MAX(CASE
                WHEN o.user_mark = 'wrong' THEN 100
                WHEN o.mark_level = 'unknown'
                    OR (o.mark_level = 'none' AND o.user_mark = 'unknown') THEN 80
                WHEN o.mark_level = 'familiar' THEN 55
                WHEN o.mark_level = 'fuzzy'
                    OR (o.mark_level = 'none' AND o.user_mark = 'ignored') THEN 35
                ELSE 0
            END)
            FROM exercise_vocab_occurrences o
            WHERE o.entry_id = {entry_id_expression}
        ), 0)"
    )
}

fn load_unlearned_ranked_entry_ids(
    conn: &word_storage_core::Connection,
    limit: usize,
) -> Result<Vec<i64>, String> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let mark_priority = exercise_vocab_mark_priority_expression("e.id");
    let sql = format!(
        "SELECT e.id
         FROM entries e
         WHERE NOT EXISTS (
           SELECT 1
           FROM study_results sr
           WHERE sr.entry_id = e.id
         )
         ORDER BY {mark_priority} DESC,
                  e.frequency DESC, e.id ASC
         LIMIT {limit}"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare unlearned fallback entry seed query: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(|e| format!("Failed to query unlearned fallback entry seed: {e}"))?;
    let ids = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode unlearned fallback entry seed row: {e}"))?;
    filter_mastered_entry_ids(conn, ids)
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
    let ids = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode random wordbook entry seed row: {e}"))?;
    filter_mastered_entry_ids(conn, ids)
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
    let ids = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode wordbook entry seed row: {e}"))?;
    filter_mastered_entry_ids(conn, ids)
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
    let mut ids = ranked
        .into_iter()
        .filter(|id| !excluded_entry_ids.contains(id))
        .take(limit)
        .collect::<Vec<_>>();

    if !wordbook_ids.is_empty() && ids.len() < limit {
        let fallback_limit = limit
            .saturating_add(excluded_entry_ids.len())
            .saturating_mul(2)
            .max(limit);
        for id in load_ranked_entry_ids(conn, fallback_limit)? {
            if excluded_entry_ids.contains(&id) || ids.contains(&id) {
                continue;
            }
            ids.push(id);
            if ids.len() >= limit {
                break;
            }
        }
    }

    Ok(ids)
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
    let ids = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode fallback entry seed row: {e}"))?;
    filter_mastered_entry_ids(conn, ids)
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
    let ids = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to decode random fallback entry seed row: {e}"))?;
    filter_mastered_entry_ids(conn, ids)
}

fn load_entry_payloads(
    conn: &word_storage_core::Connection,
    entry_ids: &[i64],
) -> Result<Vec<StartSessionEntryPayload>, String> {
    let question_preps = load_entry_question_preps_map(conn, entry_ids)?;
    let mut payloads = Vec::with_capacity(entry_ids.len());
    for entry_id in entry_ids {
        let preps = question_preps
            .get(entry_id)
            .cloned()
            .unwrap_or_else(|| (Vec::new(), Vec::new()));
        let payload = load_entry_payload_with_question_preps(conn, *entry_id, preps)?;
        payloads.push(payload);
    }
    Ok(payloads)
}

fn load_first_valid_study_payload_excluding_sources(
    conn: &word_storage_core::Connection,
    candidate_entry_ids: &[i64],
    excluded_source_ids: &BTreeSet<String>,
) -> Result<Option<StartSessionEntryPayload>, String> {
    for entry_id in candidate_entry_ids {
        let source_id = conn
            .query_row(
                "SELECT source_entry_key FROM entries WHERE id = ?1",
                [entry_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(|error| format!("Failed to load replacement source id: {error}"))?;
        if excluded_source_ids.contains(&source_id) {
            continue;
        }

        let mut question_preps = load_entry_question_preps_map(conn, &[*entry_id])?;
        let preps = question_preps
            .remove(entry_id)
            .unwrap_or_else(|| (Vec::new(), Vec::new()));
        let payload = load_entry_payload_with_question_preps(conn, *entry_id, preps)?;
        if is_valid_bridge_study_payload(&payload) {
            return Ok(Some(payload));
        }
    }
    Ok(None)
}

#[cfg(test)]
fn load_entry_payload(
    conn: &word_storage_core::Connection,
    entry_id: i64,
) -> Result<StartSessionEntryPayload, String> {
    let preps = load_entry_question_preps(conn, entry_id)?;
    load_entry_payload_with_question_preps(conn, entry_id, preps)
}

fn load_entry_payload_with_question_preps(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    question_preps: (Vec<String>, Vec<String>),
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
    let mut meaning_details = meaning_details;
    append_user_accepted_meaning_payloads(conn, entry_id, &mut meaning_details)?;
    let meanings = meaning_details
        .iter()
        .map(|meaning| meaning.meaning_cn.clone())
        .collect::<Vec<_>>();
    let examples = load_entry_examples(conn, entry_id)?;
    let selected_example =
        select_entry_payload_example(&examples, &meaning_details, part_of_speech.as_deref());
    let example_sentence = selected_example
        .as_ref()
        .and_then(|value| value.get("sentenceEn"))
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let example_translation = selected_example
        .as_ref()
        .and_then(|value| value.get("sentenceCn"))
        .and_then(|value| value.as_str())
        .map(str::to_string);

    let (cn_choice_distractors, en_choice_distractors) = question_preps;

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
        cn_choice_distractors,
        en_choice_distractors,
    })
}

fn load_entry_question_preps_map(
    conn: &word_storage_core::Connection,
    entry_ids: &[i64],
) -> Result<BTreeMap<i64, (Vec<String>, Vec<String>)>, String> {
    let mut out = BTreeMap::new();
    if entry_ids.is_empty() {
        return Ok(out);
    }

    let placeholders = std::iter::repeat("?")
        .take(entry_ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT entry_id, cn_choice_distractors_json, en_choice_distractors_json
         FROM entry_question_preps
         WHERE entry_id IN ({placeholders})"
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to prepare question preps batch query: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(entry_ids.iter()), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("Failed to query question preps batch: {e}"))?;

    for row in rows {
        let (entry_id, cn_json, en_json) =
            row.map_err(|e| format!("Failed to decode question preps batch row: {e}"))?;
        out.insert(
            entry_id,
            (
                serde_json::from_str::<Vec<String>>(&cn_json).unwrap_or_default(),
                serde_json::from_str::<Vec<String>>(&en_json).unwrap_or_default(),
            ),
        );
    }

    Ok(out)
}
#[cfg(test)]
fn load_entry_question_preps(
    conn: &word_storage_core::Connection,
    entry_id: i64,
) -> Result<(Vec<String>, Vec<String>), String> {
    let row = conn
        .query_row(
            "SELECT cn_choice_distractors_json, en_choice_distractors_json
             FROM entry_question_preps
             WHERE entry_id = ?1",
            [entry_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|e| format!("Failed to load question preps: {e}"))?;
    let Some((cn_json, en_json)) = row else {
        return Ok((Vec::new(), Vec::new()));
    };
    let cn_choice_distractors = serde_json::from_str::<Vec<String>>(&cn_json).unwrap_or_default();
    let en_choice_distractors = serde_json::from_str::<Vec<String>>(&en_json).unwrap_or_default();
    Ok((cn_choice_distractors, en_choice_distractors))
}
fn append_user_accepted_meaning_payloads(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    meaning_details: &mut Vec<StartSessionMeaningPayload>,
) -> Result<(), String> {
    let existing = meaning_details
        .iter()
        .map(|meaning| normalize_seed_meaning_for_comparison(&meaning.meaning_cn))
        .collect::<BTreeSet<_>>();
    let mut seen = existing;
    let accepted = persistence::user_accepted_meaning_repo::list_for_entry(conn, entry_id)
        .map_err(|e| format!("Failed to load user accepted meanings: {e}"))?;
    for meaning in accepted {
        let text = clean_seed_meaning_cn(&meaning.meaning_cn);
        let key = normalize_seed_meaning_for_comparison(&text);
        if text.is_empty() || key.is_empty() || !seen.insert(key) {
            continue;
        }
        meaning_details.push(StartSessionMeaningPayload {
            pos: "user_dispute".to_string(),
            meaning_cn: text,
            meaning_en: None,
        });
    }
    Ok(())
}

fn append_user_accepted_meaning_strings(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    meanings: &mut Vec<serde_json::Value>,
) -> Result<(), String> {
    let mut seen = meanings
        .iter()
        .filter_map(|meaning| meaning.as_str())
        .map(normalize_seed_meaning_for_comparison)
        .collect::<BTreeSet<_>>();
    let accepted = persistence::user_accepted_meaning_repo::list_for_entry(conn, entry_id)
        .map_err(|e| format!("Failed to load user accepted meanings: {e}"))?;
    for meaning in accepted {
        let text = clean_seed_meaning_cn(&meaning.meaning_cn);
        let key = normalize_seed_meaning_for_comparison(&text);
        if text.is_empty() || key.is_empty() || !seen.insert(key) {
            continue;
        }
        meanings.push(serde_json::Value::String(text));
    }
    Ok(())
}

fn append_user_accepted_meaning_details(
    conn: &word_storage_core::Connection,
    entry_id: i64,
    meaning_details: &mut Vec<serde_json::Value>,
) -> Result<(), String> {
    let mut seen = meaning_details
        .iter()
        .filter_map(|meaning| meaning.get("meaningCn").and_then(|value| value.as_str()))
        .map(normalize_seed_meaning_for_comparison)
        .collect::<BTreeSet<_>>();
    let accepted = persistence::user_accepted_meaning_repo::list_for_entry(conn, entry_id)
        .map_err(|e| format!("Failed to load user accepted meanings: {e}"))?;
    for meaning in accepted {
        let text = clean_seed_meaning_cn(&meaning.meaning_cn);
        let key = normalize_seed_meaning_for_comparison(&text);
        if text.is_empty() || key.is_empty() || !seen.insert(key) {
            continue;
        }
        meaning_details.push(serde_json::json!({
            "pos": "user_dispute",
            "meaningCn": text,
            "meaningEn": null
        }));
    }
    Ok(())
}

fn normalize_seed_meaning_for_comparison(value: &str) -> String {
    clean_seed_meaning_cn(value)
        .chars()
        .filter(|ch| {
            !matches!(
                ch,
                ' ' | '\t'
                    | '\n'
                    | '\r'
                    | ','
                    | ';'
                    | '/'
                    | '\u{3001}'
                    | '\u{3002}'
                    | '\u{ff0c}'
                    | '\u{ff1b}'
                    | '\u{ff1a}'
            )
        })
        .collect()
}

fn select_entry_payload_example(
    examples: &[serde_json::Value],
    meaning_details: &[StartSessionMeaningPayload],
    part_of_speech: Option<&str>,
) -> Option<serde_json::Value> {
    let target_pos = part_of_speech
        .map(normalize_seed_pos_key)
        .unwrap_or_default();
    let target_meanings = meaning_details
        .iter()
        .filter(|meaning| {
            target_pos.is_empty() || normalize_seed_pos_key(&meaning.pos) == target_pos
        })
        .map(|meaning| meaning.meaning_cn.as_str())
        .collect::<Vec<_>>();
    let candidate_meanings = if target_meanings.is_empty() {
        meaning_details
            .iter()
            .map(|meaning| meaning.meaning_cn.as_str())
            .collect::<Vec<_>>()
    } else {
        target_meanings
    };

    examples
        .iter()
        .filter(|example| {
            example
                .get("sentenceCn")
                .and_then(|value| value.as_str())
                .is_some_and(is_real_exam_source_label)
        })
        .next()
        .cloned()
        .or_else(|| {
            examples
                .iter()
                .filter_map(|example| {
                    let translation = example
                        .get("sentenceCn")
                        .and_then(|value| value.as_str())
                        .unwrap_or("");
                    let score = candidate_meanings
                        .iter()
                        .map(|meaning| seed_text_overlap_score(translation, meaning))
                        .max()
                        .unwrap_or(0);
                    if score > 0 {
                        Some((score, example.clone()))
                    } else {
                        None
                    }
                })
                .max_by_key(|(score, _)| *score)
                .map(|(_, example)| example)
                .or_else(|| examples.first().cloned())
        })
}

fn is_real_exam_source_label(value: &str) -> bool {
    value
        .trim_start()
        .starts_with("\u{771f}\u{9898}\u{6765}\u{6e90}\u{ff1a}")
}

fn seed_text_overlap_score(left: &str, right: &str) -> usize {
    let left = normalize_seed_text_for_overlap(left);
    let right = normalize_seed_text_for_overlap(right);
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    if left.contains(&right) {
        return 100 + right.chars().count();
    }
    let left_chars = left.chars().collect::<BTreeSet<_>>();
    right.chars().filter(|ch| left_chars.contains(ch)).count()
}

fn normalize_seed_text_for_overlap(value: &str) -> String {
    value
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    ',' | '.'
                        | ';'
                        | ':'
                        | '/'
                        | '\u{3001}'
                        | '\u{3002}'
                        | '\u{FF0C}'
                        | '\u{FF1B}'
                        | '\u{FF1A}'
                )
        })
        .collect()
}

/// Submit a study answer.
///
/// Takes a JSON string containing SubmitAnswerRequest.
/// Returns a JSON string containing SubmitAnswerResponse.
pub fn submit_study_answer(request_json: String) -> Result<String, String> {
    let request_value = serde_json::from_str::<serde_json::Value>(&request_json).ok();
    let hint_used = request_value
        .as_ref()
        .and_then(|value| value.get("hintUsed").and_then(|field| field.as_bool()))
        .unwrap_or(false);
    let include_synonyms = request_value
        .as_ref()
        .and_then(|value| value.get("studyMode").and_then(|field| field.as_str()))
        .is_some_and(|mode| mode == "highFrequency");
    let request: SubmitAnswerRequest =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;

    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let conn = open_study_session_action_database(runtime)?;

    let response = word_app_core::facade::study_facade::submit_study_answer_with_hint_usage(
        &conn, request, hint_used,
    )
    .map_err(|e| format!("Failed to submit answer: {}", e))?;
    enqueue_study_sync_snapshots_after_submit(
        &conn,
        Some((&response.result.question_id, &response.result.answered_at)),
        response.is_complete,
    );

    let mut payload =
        serde_json::to_value(&response).map_err(|e| format!("JSON serialization failed: {e}"))?;
    enrich_submit_response_hints(&conn, &mut payload, include_synonyms)?;
    enrich_mastered_entry_count(&conn, &mut payload)?;
    clean_json_string(&payload)
}

pub fn mark_study_entry_mastered(request_json: String) -> Result<String, String> {
    let request: word_storage_core::models::MarkStudyEntryMasteredRequest =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;

    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let conn = open_study_session_action_database(runtime)?;

    let replacements = load_mastered_replacement_payloads(
        &conn,
        &request,
        Some(runtime.paths().bundled_resource_path("")),
    )?;
    let response = core_mark_study_entry_mastered_with_replacements(&conn, request, replacements)
        .map_err(|e| format!("Failed to mark study entry mastered: {}", e))?;
    enqueue_study_sync_snapshots_after_submit(&conn, None, response.is_complete);

    let mut payload =
        serde_json::to_value(&response).map_err(|e| format!("JSON serialization failed: {e}"))?;
    enrich_study_response_hints(&conn, &mut payload)?;
    enrich_mastered_entry_count(&conn, &mut payload)?;
    clean_json_string(&payload)
}

fn enrich_mastered_entry_count(
    conn: &word_storage_core::Connection,
    payload: &mut serde_json::Value,
) -> Result<(), String> {
    let count = persistence::mastered_entry_repo::mastered_entry_count(conn)
        .map_err(|error| format!("Failed to count mastered entries: {error}"))?;
    let object = payload
        .as_object_mut()
        .ok_or_else(|| "Study response must be a JSON object".to_string())?;
    object.insert("masteredCount".to_string(), serde_json::json!(count));
    Ok(())
}

struct MasteredReplacementContext {
    mode: SessionMode,
    wordbook_id: Option<i64>,
    entry_source_ids: BTreeSet<String>,
    question_type_weights: Vec<QuestionTypeWeight>,
    entry_was_answered: bool,
}

fn load_mastered_replacement_payloads(
    conn: &word_storage_core::Connection,
    request: &word_storage_core::models::MarkStudyEntryMasteredRequest,
    bundle_resource_dir: Option<std::path::PathBuf>,
) -> Result<Vec<StartSessionEntryPayload>, String> {
    let Some(context) = load_mastered_replacement_context(conn, &request.entry_source_id)? else {
        return Ok(Vec::new());
    };
    if context.entry_was_answered {
        return Ok(Vec::new());
    }

    persistence::mastered_entry_repo::mark_mastered_by_source_id(
        conn,
        &request.entry_source_id,
        &request.reason,
    )
    .map_err(|error| format!("Failed to prepare mastered replacement: {error}"))?;

    if matches!(context.mode, SessionMode::HighFrequency) {
        let candidate_limit =
            study_entry_candidate_limit(context.entry_source_ids.len().saturating_add(1));
        let candidate_ids = load_high_frequency_candidate_entry_ids(conn, candidate_limit)?;
        return Ok(load_first_valid_study_payload_excluding_sources(
            conn,
            &candidate_ids,
            &context.entry_source_ids,
        )?
        .into_iter()
        .collect());
    }

    let hydrated = hydrate_start_session_request(
        conn,
        StartSessionRequest {
            mode: context.mode,
            wordbook_id: context.wordbook_id,
            entry_source_ids: Vec::new(),
            entry_payloads: Vec::new(),
            distractor_payloads: Vec::new(),
            question_type_weights: context.question_type_weights,
        },
        bundle_resource_dir,
    )?;
    Ok(hydrated
        .entry_payloads
        .into_iter()
        .find(|payload| !context.entry_source_ids.contains(&payload.source_id))
        .into_iter()
        .collect())
}

fn load_mastered_replacement_context(
    conn: &word_storage_core::Connection,
    entry_source_id: &str,
) -> Result<Option<MasteredReplacementContext>, String> {
    let modes = [
        SessionMode::NewWord,
        SessionMode::Review,
        SessionMode::MixedTest,
        SessionMode::WrongWordReinforcement,
        SessionMode::HighFrequency,
        SessionMode::RootAffix,
    ];
    for mode in modes {
        let Some(raw) = persistence::study_repo::load_active_session_snapshot(conn, &mode)
            .map_err(|error| format!("Failed to load mastered replacement snapshot: {error}"))?
        else {
            continue;
        };
        let snapshot: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|error| format!("Invalid mastered replacement snapshot: {error}"))?;
        let mut entry_source_ids = snapshot
            .get("entrySourceIds")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        entry_source_ids.extend(
            snapshot
                .get("entryPayloads")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|payload| payload.get("sourceId"))
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string),
        );
        if !entry_source_ids.contains(entry_source_id) {
            continue;
        }
        let entry_was_answered = snapshot
            .get("results")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .any(|result| {
                result
                    .get("entrySourceId")
                    .and_then(serde_json::Value::as_str)
                    == Some(entry_source_id)
            });
        let wordbook_id = snapshot
            .get("session")
            .and_then(|session| session.get("wordbookId"))
            .and_then(serde_json::Value::as_i64);
        let question_type_weights = snapshot
            .get("questionTypeWeights")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        return Ok(Some(MasteredReplacementContext {
            mode,
            wordbook_id,
            entry_source_ids,
            question_type_weights,
            entry_was_answered,
        }));
    }
    Ok(None)
}

pub fn accept_disputed_meaning(request_json: String) -> Result<String, String> {
    let request: AcceptDisputedMeaningRequest =
        serde_json::from_str(&request_json).map_err(|e| format!("Invalid request: {}", e))?;

    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let conn = open_study_session_action_database(runtime)?;

    let response = core_accept_disputed_meaning(&conn, request)
        .map_err(|e| format!("Failed to accept disputed meaning: {}", e))?;
    let outbox_payload = serde_json::json!({
        "type": "word_disputed_meaning",
        "entrySourceId": response.entry_source_id,
        "word": response.word,
        "submittedMeaning": response.accepted_meaning,
        "questionId": response.result.question_id,
        "questionType": response.result.question_type,
        "acceptedAt": response.result.answered_at,
    });
    enqueue_sync_snapshot(
        &conn,
        "word_disputed_meaning",
        &outbox_payload,
        &format!(
            "word_disputed_meaning:{}:{}",
            response.entry_source_id, response.result.question_id
        ),
    );
    if should_enqueue_study_sync_snapshots_after_dispute_accept() {
        enqueue_recent_study_word_points(&conn);
        enqueue_wrong_word_entries_snapshot(&conn);
    }

    let mut payload =
        serde_json::to_value(&response).map_err(|e| format!("JSON serialization failed: {e}"))?;
    enrich_study_response_hints(&conn, &mut payload)?;
    clean_json_string(&payload)
}

/// Complete the current study session.
///
/// Takes a session ID string.
/// Returns a JSON string containing CompleteSessionResponse.
pub fn complete_study_session(session_id: String) -> Result<String, String> {
    let runtime_guard = get_runtime()?;
    let runtime = runtime_guard.as_ref().ok_or("Runtime not initialized")?;

    let conn = open_study_session_action_database(runtime)?;

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

    let conn = open_study_session_action_database(runtime)?;

    core_cancel_study_session(&conn, &session_id)
        .map_err(|e| format!("Failed to cancel session: {}", e))
}

#[cfg(test)]
mod tests {
    use super::{
        active_wordbook_ids_from_selection, align_today_targets_to_active_sessions,
        align_today_targets_to_available_pools, align_today_targets_to_completed_sessions,
        append_other_wrong_words_block, build_all_study_events_payload,
        build_authoritative_today_home_state, build_wrong_word_entries_payload,
        build_wrong_word_image_user_prompt, clear_persisted_active_study_sessions,
        commit_wrong_word_import_with_connection, create_reward_image_upload_with_connection,
        enqueue_study_sync_snapshots_after_submit, enrich_root_affix_entries_from_assets,
        enrich_study_question_hints, enrich_submit_response_hints, ensure_planning_state,
        ensure_seed_vocabulary_available_for_today, ensure_seed_vocabulary_imported,
        get_json_setting, hydrate_start_session_request, list_reward_images_with_connection,
        load_entry_payload, load_high_frequency_candidate_entry_ids,
        load_learned_entry_ids_for_wordbooks, load_mastered_replacement_context,
        load_mastered_replacement_payloads, load_prioritized_wrong_word_entry_ids,
        load_review_entry_ids_for_today, load_reward_image_upload_entitlement,
        load_root_affix_payloads_for_active_wordbooks_on_date, load_today_completion_seed,
        load_unlearned_ranked_entry_ids_for_wordbooks_on_date,
        load_valid_study_payloads_for_candidates, load_wrong_word_detail_payload_with_bundle,
        load_wrong_word_entries, load_wrong_word_inputs, merge_cloud_study_events,
        moderate_reward_image_with_connection, normalize_question_type_weights,
        normalize_stored_session_mode, parse_wrong_word_import_ai_output,
        rebuild_seed_question_preps, refresh_reward_image_entitlement_with_connection,
        repair_seed_vocabulary_dedup, reset_user_owned_local_data,
        restore_cloud_wordbook_preferences, restore_cloud_wrong_word_hints,
        seed_local_leaderboard_demo_with_connection, select_review_candidates,
        selected_review_wordbook_ids_for_today, selected_wordbook_ids_for_today, set_json_setting,
        should_apply_schema_on_study_session_action,
        should_enqueue_study_sync_snapshots_after_dispute_accept,
        should_replace_today_review_wordbooks, single_wordbook_selection_json,
        snapshot_answered_question_count, split_ai_passage_wrong_words, study_mode_target_counts,
        today_date_string, today_target_seed_from_plan_value,
        today_target_seed_from_plan_value_for_date, try_resume_empty_start_request,
        vote_reward_image_with_connection, with_runtime_conn, ReviewCandidate, TodayTargetSeed,
        AI_PASSAGE_BODY_WORD_LIMIT, AI_PASSAGE_MAX_WRONG_WORDS, WRONG_WORD_IMPORT_PROMPT_TEMPLATE,
    };
    use crate::ai_agent::{AiAgent, AiProviderConfig, AiProviderProfile};
    use rusqlite::Connection;
    use std::env;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn shared_study_events_merge_once_without_reentering_mobile_aggregates() {
        let conn = Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute("INSERT INTO source_versions (id, source_commit, status) VALUES (1, 'shared-event-test', 'ready')", []).expect("source");
        conn.execute("INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma) VALUES (1, 1, 'KaoYan_3_1', 'process', 'process')", []).expect("entry");
        conn.execute("INSERT INTO wordbooks (id, code, name, source_book_id, source_version_id, total_entries) VALUES (1, 'kaoyan', 'Kaoyan', 'kaoyan', 1, 1)", []).expect("book");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book) VALUES (1, 1, 1)",
            [],
        )
        .expect("book entry");
        let events = serde_json::json!([{
            "event_type": "answer_submitted",
            "session_id": "web-session",
            "occurred_at": "2026-08-21T08:00:00+00:00",
            "payload_json": {
                "schemaVersion": 1, "eventId": "web-1", "questionId": "q-1",
                "entryId": 1, "entrySourceId": "KaoYan_3_1", "bookId": "kaoyan", "bookVersion": "2026.3",
                "mode": "highFrequency", "questionType": "enToCnInput", "outcome": "correct",
                "userResponse": "过程", "canonicalAnswer": "过程；进程",
                "responseTimeMs": 1200, "hintUsed": true
            }
        }]);

        assert_eq!(merge_cloud_study_events(&conn, Some(&events)).unwrap(), 1);
        assert_eq!(merge_cloud_study_events(&conn, Some(&events)).unwrap(), 0);
        let result: (String, i64) = conn.query_row(
            "SELECT outcome, hint_used FROM study_results WHERE question_id LIKE 'cloud_event:%'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).expect("merged result");
        assert_eq!(result, ("correct".to_string(), 1));
        let aggregate_candidates: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM study_results WHERE session_id NOT LIKE 'cloud_event:%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(aggregate_candidates, 0);
    }

    #[test]
    fn mobile_backfill_exports_all_local_answers_as_shared_events() {
        let conn = Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute("INSERT INTO source_versions (id, source_commit, status) VALUES (1, 'mobile-event-test', 'ready')", []).expect("source");
        conn.execute("INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma) VALUES (1, 1, 'KaoYan_3_1', 'process', 'process')", []).expect("entry");
        conn.execute("INSERT INTO wordbooks (id, code, name, source_book_id, source_version_id, total_entries) VALUES (1, 'kaoyan', 'Kaoyan', 'kaoyan', 1, 1)", []).expect("book");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book) VALUES (1, 1, 1)",
            [],
        )
        .expect("book entry");
        conn.execute("INSERT INTO study_sessions (session_id, mode, total_words, wordbook_id, started_at, completed_at) VALUES ('local-old', 'highFrequency', 1, 1, '2024-01-01T00:00:00Z', '2024-01-01T00:01:00Z'), ('cloud_event:web', 'review', 1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:01:00Z')", []).expect("sessions");
        conn.execute("INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response, correct_answer, outcome, response_time_ms, answered_at, hint_used) VALUES ('local-old', 'q-local', 1, 'exampleComprehensionChoice', '过程', '过程；进程', 'correct', 1200, '2024-01-01T00:00:30Z', 1), ('cloud_event:web', 'cloud_event:web:q', 1, 'enToCnInput', '过程', '过程；进程', 'correct', 900, '2026-01-01T00:00:30Z', 0)", []).expect("results");

        let payload = build_all_study_events_payload(&conn)
            .expect("event payload")
            .expect("events");
        let events = payload
            .get("events")
            .and_then(|value| value.as_array())
            .expect("events array");
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0]
                .get("localResultId")
                .and_then(|value| value.as_i64()),
            Some(1)
        );
        assert_eq!(
            events[0]
                .pointer("/payloadJson/localDay")
                .and_then(|value| value.as_str()),
            Some("2024-01-01")
        );
        assert_eq!(
            events[0]
                .pointer("/payloadJson/bookId")
                .and_then(|value| value.as_str()),
            Some("kaoyan")
        );
        assert_eq!(
            events[0]
                .pointer("/payloadJson/entrySourceId")
                .and_then(|value| value.as_str()),
            Some("KaoYan_3_1")
        );
        assert_eq!(
            events[0]
                .pointer("/payloadJson/questionType")
                .and_then(|value| value.as_str()),
            Some("exampleComprehensionChoice")
        );
        assert_eq!(
            events[0]
                .pointer("/payloadJson/legacyPointProjection")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
    }

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

    #[test]
    fn study_question_enrichment_exposes_persisted_error_count() {
        let conn = Connection::open_in_memory().expect("open database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'error-count-test', 'ready')",
            [],
        )
        .expect("source");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma)
             VALUES (1, 1, 'available', 'available', 'available')",
            [],
        )
        .expect("entry");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at)
             VALUES ('s-errors', 'highFrequency', 1, '2026-08-20')",
            [],
        )
        .expect("session");
        for (index, outcome) in ["incorrect", "skipped"].into_iter().enumerate() {
            conn.execute(
                "INSERT INTO study_results (
                    session_id, question_id, entry_id, question_type, user_response,
                    correct_answer, outcome, response_time_ms, answered_at
                 ) VALUES ('s-errors', ?1, 1, 'enToCnInput', 'x', 'x', ?2, 1, ?3)",
                rusqlite::params![
                    format!("q-{index}"),
                    outcome,
                    format!("2026-08-20T00:00:0{index}Z")
                ],
            )
            .expect("result");
        }
        let mut question = serde_json::json!({
            "entrySourceId": "available",
            "word": "available"
        });

        enrich_study_question_hints(&conn, &mut question).expect("enrich question");

        assert_eq!(question["errorCount"], serde_json::json!(2));
    }

    #[test]
    fn high_frequency_candidates_expand_beyond_the_daily_plan_without_a_fixed_pool() {
        let conn = Connection::open_in_memory().expect("open database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status) VALUES (1, 'hf-test', 'ready')",
            [],
        )
        .expect("source");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, category, source_version_id) VALUES (3, 'kaoyan', 'KaoYan', 'exam', 1)",
            [],
        )
        .expect("wordbook");
        for rank in 1..=160i64 {
            conn.execute(
                "INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma, exam_frequency, exam_rank)
                 VALUES (?1, 1, ?2, ?2, ?2, ?3, ?1)",
                rusqlite::params![rank, format!("word-{rank}"), 200 - rank],
            )
            .expect("entry");
            conn.execute(
                "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book) VALUES (3, ?1, ?1)",
                [rank],
            )
            .expect("wordbook entry");
        }

        set_json_setting(
            &conn,
            "high_frequency_pool_json",
            &serde_json::json!([1, 2, 3]),
        )
        .expect("legacy pool");

        let drawn = load_high_frequency_candidate_entry_ids(&conn, 101).expect("dynamic draw");
        assert_eq!(drawn.len(), 101);
        let legacy = get_json_setting(&conn, "high_frequency_pool_json", &serde_json::json!([]))
            .expect("legacy pool remains untouched");
        assert_eq!(legacy, serde_json::json!([1, 2, 3]));

        for entry_id in 1..=50i64 {
            conn.execute(
                "INSERT INTO mastered_entries (source_entry_id, entry_id, reason) VALUES (?1, ?2, 'test')",
                rusqlite::params![format!("word-{entry_id}"), entry_id],
            )
            .expect("mastered");
        }
        let shifted = load_high_frequency_candidate_entry_ids(&conn, 101).expect("shifted draw");
        assert_eq!(shifted.len(), 101);
        assert!(shifted.iter().all(|entry_id| *entry_id > 50));
    }

    #[test]
    fn unanswered_high_frequency_mastered_action_selects_out_of_session_replacement() {
        let conn = Connection::open_in_memory().expect("open database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status) VALUES (1, 'hf-replace', 'ready')",
            [],
        )
        .expect("source");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, category, source_version_id) VALUES (3, 'kaoyan', 'KaoYan', 'exam', 1)",
            [],
        )
        .expect("wordbook");
        for rank in 1..=120i64 {
            conn.execute(
                "INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma, part_of_speech, exam_frequency, exam_rank)
                 VALUES (?1, 1, ?2, ?2, ?2, 'n', ?3, ?1)",
                rusqlite::params![rank, format!("word-{rank}"), 200 - rank],
            )
            .expect("entry");
            conn.execute(
                "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order) VALUES (?1, 'n', ?2, 0)",
                rusqlite::params![rank, format!("meaning-{rank}")],
            )
            .expect("meaning");
            conn.execute(
                "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book) VALUES (3, ?1, ?1)",
                [rank],
            )
            .expect("wordbook entry");
        }
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({"highFrequencyPerDay": 100}),
        )
        .expect("plan");
        set_json_setting(
            &conn,
            "high_frequency_pool_json",
            &serde_json::json!((1..=100).collect::<Vec<_>>()),
        )
        .expect("legacy fixed pool");
        let entry_source_ids = (1..=100)
            .map(|rank| format!("word-{rank}"))
            .collect::<Vec<_>>();
        set_json_setting(
            &conn,
            "active_study_session_highFrequency",
            &serde_json::json!({
                "session": {
                    "sessionId": "hf-replacement-session",
                    "mode": "highFrequency",
                    "totalWords": 100,
                    "wordbookId": 3,
                    "startedAt": chrono::Utc::now().to_rfc3339()
                },
                "entrySourceIds": entry_source_ids,
                "entryPayloads": [],
                "results": [],
                "questionTypeWeights": []
            }),
        )
        .expect("active snapshot");

        let context = load_mastered_replacement_context(&conn, "word-100")
            .expect("replacement context")
            .expect("active high-frequency context");
        assert!(matches!(
            context.mode,
            word_storage_core::models::SessionMode::HighFrequency
        ));
        assert_eq!(context.entry_source_ids.len(), 100);

        let replacements = load_mastered_replacement_payloads(
            &conn,
            &word_storage_core::models::MarkStudyEntryMasteredRequest {
                entry_source_id: "word-100".to_string(),
                reason: "mastered".to_string(),
            },
            None,
        )
        .expect("replacement payload");

        let candidates = load_high_frequency_candidate_entry_ids(&conn, 200)
            .expect("post-mastered dynamic candidates");
        assert_eq!(candidates.len(), 119);
        let (_, valid_candidates) =
            load_valid_study_payloads_for_candidates(&conn, &candidates, candidates.len())
                .expect("valid dynamic candidates");
        assert!(valid_candidates
            .iter()
            .any(|payload| !entry_source_ids.contains(&payload.source_id)));

        assert_eq!(replacements.len(), 1);
        assert!(!entry_source_ids.contains(&replacements[0].source_id));
        assert!(
            word_storage_core::persistence::mastered_entry_repo::is_mastered_source_id(
                &conn, "word-100"
            )
            .expect("mastered state")
        );

        set_json_setting(
            &conn,
            "active_study_session_highFrequency",
            &serde_json::json!({
                "session": {
                    "sessionId": "hf-replacement-session",
                    "mode": "highFrequency",
                    "totalWords": 100,
                    "wordbookId": 3,
                    "startedAt": chrono::Utc::now().to_rfc3339()
                },
                "entrySourceIds": entry_source_ids,
                "entryPayloads": [],
                "results": [{"entrySourceId": "word-9"}],
                "questionTypeWeights": []
            }),
        )
        .expect("answered active snapshot");
        let answered_replacements = load_mastered_replacement_payloads(
            &conn,
            &word_storage_core::models::MarkStudyEntryMasteredRequest {
                entry_source_id: "word-9".to_string(),
                reason: "mastered".to_string(),
            },
            None,
        )
        .expect("answered replacement payload");
        assert!(answered_replacements.is_empty());
        assert!(
            !word_storage_core::persistence::mastered_entry_repo::is_mastered_source_id(
                &conn, "word-9"
            )
            .expect("answered prefetch mastered state")
        );
    }

    #[test]
    fn high_frequency_restart_uses_only_unfinished_daily_target() {
        let conn = Connection::open_in_memory().expect("open database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        let entry_id = seed_basic_entry(&conn, "remaining", "remaining meaning");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({"highFrequencyPerDay": 50}),
        )
        .expect("plan");
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('hf-partial', '\"highFrequency\"', 50, ?1, ?1)",
            [&now],
        )
        .expect("partial session");
        for index in 0..49 {
            conn.execute(
                "INSERT INTO study_results (session_id, question_id, entry_id, question_type,
                 user_response, correct_answer, outcome, response_time_ms, answered_at)
                 VALUES ('hf-partial', ?1, ?2, '\"enToCnInput\"', 'x', 'x', '\"correct\"', 1, ?3)",
                rusqlite::params![format!("hf-q-{index}"), entry_id, now],
            )
            .expect("partial result");
        }

        let (remaining, planned) = study_mode_target_counts(
            &conn,
            &word_storage_core::models::SessionMode::HighFrequency,
        )
        .expect("target counts");
        assert_eq!(planned, 50);
        assert_eq!(remaining, 1);
    }

    #[test]
    fn inconsistent_active_high_frequency_snapshot_does_not_reduce_today_target() {
        let conn = Connection::open_in_memory().expect("open database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({"highFrequencyPerDay": 100}),
        )
        .expect("plan");
        let now = chrono::Utc::now().to_rfc3339();
        set_json_setting(
            &conn,
            "active_study_session_highFrequency",
            &serde_json::json!({
                "session": {"sessionId": "hf-inconsistent", "startedAt": now},
                "currentIndex": 0,
                "results": [
                    {"questionId": "hf-q-0"},
                    {"questionId": "hf-q-1"},
                    {"questionId": "hf-q-2"},
                    {"questionId": "hf-q-3"},
                    {"questionId": "hf-q-4"}
                ]
            }),
        )
        .expect("active snapshot");

        let completion =
            load_today_completion_seed(&conn, &today_date_string()).expect("today completion");
        assert_eq!(completion.high_frequency_completed, 0);

        let (remaining, planned) = study_mode_target_counts(
            &conn,
            &word_storage_core::models::SessionMode::HighFrequency,
        )
        .expect("target counts");
        assert_eq!(planned, 100);
        assert_eq!(remaining, 100);
    }

    #[test]
    fn question_type_weights_are_normalized_and_mode_scoped() {
        let normalized = normalize_question_type_weights(&serde_json::json!({
            "mixedTest": {
                "enToCnChoice": 70,
                "wordSkeletonInput": 30,
                "unknownQuestion": 1000
            },
            "rootAffix": {
                "rootToGlossInput": 10,
                "wordSkeletonInput": 90
            },
            "review": {
                "enToCnInput": -10,
                "wordSkeletonInput": 0
            }
        }));

        let mixed = normalized["mixedTest"].as_object().expect("mixed weights");
        assert_eq!(
            mixed
                .values()
                .filter_map(|value| value.as_i64())
                .sum::<i64>(),
            100
        );
        assert!(mixed.contains_key("enToCnChoice"));
        assert!(mixed.contains_key("wordSkeletonInput"));
        assert!(!mixed.contains_key("unknownQuestion"));

        assert!(normalized.get("rootAffix").is_none());

        let review = normalized["review"].as_object().expect("review weights");
        assert_eq!(
            review
                .values()
                .filter_map(|value| value.as_i64())
                .sum::<i64>(),
            100
        );
    }

    fn spawn_json_server(status_line: &str, body: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("test server addr");
        let status_line = status_line.to_string();
        let body = body.to_string();
        thread::spawn(move || {
            for _ in 0..4 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut buffer = [0u8; 2048];
                let _ = stream.read(&mut buffer);
                let response = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write test response");
            }
        });
        format!("http://{addr}")
    }

    fn spawn_counted_json_server(
        status_line: &str,
        body: &str,
        request_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("test server addr");
        let status_line = status_line.to_string();
        let body = body.to_string();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept test request");
            request_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
        let _fallback_url = EnvGuard::set("ANTHROPIC_FALLBACK_BASE_URL", &primary_url);
        let _fallback_key = EnvGuard::set("ANTHROPIC_FALLBACK_AUTH_TOKEN", "fallback-test-key");
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
            anthropic_fallback: None,
            backup: AiProviderProfile {
                provider: "openaiResponses".to_string(),
                base_url: backup_url,
                model: "gpt-test".to_string(),
                auth_token: "backup-test-key".to_string(),
            },
            agnes_fallback: None,
        });

        let result = agent
            .run_text_json("system", "user")
            .expect("backup path should succeed");

        assert_eq!(result, r#"{"title":"fallback","paragraphs":["ok"]}"#);
    }

    #[test]
    fn primary_failure_uses_secondary_anthropic_before_openai_backup() {
        let primary_url = spawn_json_server("500 Internal Server Error", r#"{"error":"down"}"#);
        let secondary_url = spawn_json_server(
            "200 OK",
            r#"{"content":[{"type":"text","text":"{\"title\":\"secondary\",\"paragraphs\":[\"ok\"]}"}]}"#,
        );
        let backup_hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let backup_url = spawn_counted_json_server(
            "200 OK",
            r#"{"output":[{"content":[{"type":"output_text","text":"{\"title\":\"backup\",\"paragraphs\":[\"ok\"]}"}]}]}"#,
            backup_hits.clone(),
        );

        let agent = AiAgent::new(AiProviderConfig {
            primary: AiProviderProfile {
                provider: "anthropic".to_string(),
                base_url: primary_url,
                model: "claude-test".to_string(),
                auth_token: "primary-test-key".to_string(),
            },
            anthropic_fallback: Some(AiProviderProfile {
                provider: "anthropic".to_string(),
                base_url: secondary_url,
                model: "claude-test".to_string(),
                auth_token: "secondary-test-key".to_string(),
            }),
            backup: AiProviderProfile {
                provider: "openaiResponses".to_string(),
                base_url: backup_url,
                model: "gpt-test".to_string(),
                auth_token: "backup-test-key".to_string(),
            },
            agnes_fallback: None,
        });

        let result = agent
            .run_text_json("system", "user")
            .expect("secondary anthropic path should succeed");

        assert_eq!(result, r#"{"title":"secondary","paragraphs":["ok"]}"#);
        assert_eq!(backup_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn failed_existing_providers_fall_back_to_agnes_responses() {
        let failed_url = spawn_json_server("500 Internal Server Error", r#"{"error":"down"}"#);
        let agnes_url = spawn_json_server(
            "200 OK",
            r#"{"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{\"title\":\"agnes\",\"paragraphs\":[\"ok\"]}"}]}]}"#,
        );
        let agent = AiAgent::new(AiProviderConfig {
            primary: AiProviderProfile {
                provider: "anthropic".to_string(),
                base_url: failed_url.clone(),
                model: "claude-test".to_string(),
                auth_token: "primary-test-key".to_string(),
            },
            anthropic_fallback: None,
            backup: AiProviderProfile {
                provider: "openaiResponses".to_string(),
                base_url: failed_url,
                model: "gpt-test".to_string(),
                auth_token: "backup-test-key".to_string(),
            },
            agnes_fallback: Some(AiProviderProfile {
                provider: "openaiResponses".to_string(),
                base_url: agnes_url,
                model: "agnes-2.5-pro-alpha".to_string(),
                auth_token: "agnes-test-key".to_string(),
            }),
        });

        let result = agent
            .run_text_json("system", "user")
            .expect("Agnes fallback should succeed");

        assert_eq!(result, r#"{"title":"agnes","paragraphs":["ok"]}"#);
    }

    #[test]
    fn exam_summary_prefers_working_relays_before_balance_limited_agnes() {
        let primary_hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let primary_url = spawn_counted_json_server(
            "200 OK",
            r#"{"content":[{"type":"text","text":"{\"provider\":\"primary\"}"}]}"#,
            primary_hits.clone(),
        );
        let agnes_hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let agnes_url = spawn_counted_json_server(
            "200 OK",
            r#"{"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{\"provider\":\"agnes\"}"}]}]}"#,
            agnes_hits.clone(),
        );
        let agent = AiAgent::new(AiProviderConfig {
            primary: AiProviderProfile {
                provider: "anthropic".to_string(),
                base_url: primary_url.clone(),
                model: "claude-test".to_string(),
                auth_token: "primary-test-key".to_string(),
            },
            anthropic_fallback: None,
            backup: AiProviderProfile {
                provider: "openaiResponses".to_string(),
                base_url: primary_url,
                model: "gpt-test".to_string(),
                auth_token: "backup-test-key".to_string(),
            },
            agnes_fallback: Some(AiProviderProfile {
                provider: "openaiResponses".to_string(),
                base_url: agnes_url,
                model: "agnes-2.5-pro-alpha".to_string(),
                auth_token: "agnes-test-key".to_string(),
            }),
        });

        let result = agent
            .run_exam_summary_json("system", "user")
            .expect("working relay should own exam summaries");

        assert_eq!(result, r#"{"provider":"primary"}"#);
        assert_eq!(primary_hits.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert_eq!(agnes_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    #[test]
    fn exam_summary_uses_agnes_only_after_all_relays_fail() {
        let failed_url = spawn_json_server("500 Internal Server Error", r#"{"error":"down"}"#);
        let agnes_url = spawn_json_server(
            "200 OK",
            r#"{"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"{\"provider\":\"agnes\"}"}]}]}"#,
        );
        let agent = AiAgent::new(AiProviderConfig {
            primary: AiProviderProfile {
                provider: "anthropic".to_string(),
                base_url: failed_url.clone(),
                model: "claude-test".to_string(),
                auth_token: "primary-test-key".to_string(),
            },
            anthropic_fallback: None,
            backup: AiProviderProfile {
                provider: "openaiResponses".to_string(),
                base_url: failed_url,
                model: "gpt-test".to_string(),
                auth_token: "backup-test-key".to_string(),
            },
            agnes_fallback: Some(AiProviderProfile {
                provider: "openaiResponses".to_string(),
                base_url: agnes_url,
                model: "agnes-2.5-pro-alpha".to_string(),
                auth_token: "agnes-test-key".to_string(),
            }),
        });

        let result = agent
            .run_exam_summary_json("system", "user")
            .expect("Agnes should remain the final exam-summary fallback");

        assert_eq!(result, r#"{"provider":"agnes"}"#);
    }

    #[test]
    fn default_agnes_profile_uses_pro_responses_and_configured_key() {
        let config = super::default_ai_provider_config();
        let agnes = config.agnes_fallback.expect("Agnes fallback profile");

        assert_eq!(agnes.base_url, "https://apihub.agnes-ai.com/v1");
        assert_eq!(agnes.provider, "openaiResponses");
        assert_eq!(agnes.model, "agnes-2.5-pro-alpha");
        assert!(!agnes.auth_token.is_empty());
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
                  "meaning": "give up",
                  "occurrenceCount": 3,
                  "confidence": 0.8,
                  "isHighFrequency": true,
                  "evidence": "seen repeatedly"
                }
              ]
            }
            ```"#,
            "image",
            "photo.jpg",
        )
        .expect("parse normalized wrong-word import output");

        assert_eq!(parsed["sourceType"].as_str(), Some("image"));
        assert_eq!(parsed["sourceName"].as_str(), Some("photo.jpg"));
        assert_eq!(parsed["warnings"].as_array().unwrap().len(), 1);
        let candidates = parsed["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0]["word"].as_str(), Some("abandon"));
        assert_eq!(candidates[0]["meaning"].as_str(), Some("give up"));
        assert_eq!(candidates[0]["occurrenceCount"].as_i64(), Some(3));
        assert_eq!(candidates[0]["confidence"].as_f64(), Some(0.8));
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
    fn wrong_word_reinforcement_demotes_words_recovered_by_recent_correct_answers() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-wrong-recovery-score', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'recovered', 'recovered', 'v.', 10.0),
                    (2, 1, 'still_wrong', 'still_wrong', 'v.', 10.0)",
            [],
        )
        .expect("insert entries");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at)
             VALUES ('sess_recovered', '\"wrongWordReinforcement\"', 1, '2026-05-01T00:00:00Z'),
                    ('sess_still_wrong', '\"wrongWordReinforcement\"', 1, '2026-05-01T00:00:00Z')",
            [],
        )
        .expect("insert sessions");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES
             ('sess_recovered', 'r_wrong_1', 1, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-01T01:00:00Z'),
             ('sess_recovered', 'r_wrong_2', 1, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-01T01:01:00Z'),
             ('sess_recovered', 'r_wrong_3', 1, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-01T01:02:00Z'),
             ('sess_recovered', 'r_wrong_4', 1, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-01T01:03:00Z'),
             ('sess_recovered', 'r_wrong_5', 1, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-01T01:04:00Z'),
             ('sess_recovered', 'r_correct_1', 1, '\"enToCnChoice\"', 'B', 'B', '\"correct\"', 100, '2026-05-01T01:05:00Z'),
             ('sess_recovered', 'r_correct_2', 1, '\"enToCnChoice\"', 'B', 'B', '\"correct\"', 100, '2026-05-01T01:06:00Z'),
             ('sess_recovered', 'r_correct_3', 1, '\"enToCnChoice\"', 'B', 'B', '\"correct\"', 100, '2026-05-01T01:07:00Z'),
             ('sess_still_wrong', 's_wrong_1', 2, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-01T01:08:00Z'),
             ('sess_still_wrong', 's_wrong_2', 2, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-01T01:09:00Z')",
            [],
        )
        .expect("insert study results");

        let ids = load_prioritized_wrong_word_entry_ids(&conn, 2).expect("load reinforcement ids");
        assert_eq!(ids, vec![2, 1]);

        let mut entries = load_wrong_word_entries(&conn).expect("load wrong entries");
        let wrong_words =
            word_app_core::services::wrong_words_service::build_wrong_words(&mut entries, "all");
        let recovered = wrong_words
            .iter()
            .find(|entry| entry["word"] == "recovered")
            .expect("recovered entry");
        assert_eq!(recovered["correctSinceLastWrong"], 3);
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
                        "trans": [{"tranCn": "\u{5438}\u{6536}"}],
                        "remMethod": {"val": "abs(\u{8fdc}\u{79bb}) + orb -> absorb"}
                    }}}
                },
                {
                    "headWord": "abstain",
                    "content": {"word": {"wordHead": "abstain", "content": {
                        "trans": [{"tranCn": "\u{5f03}\u{6743}"}],
                        "remMethod": {"val": "abs(\u{8fdc}\u{79bb}) + tain -> abstain"}
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
             VALUES ('sess_root_affix', 'q_root', 1, '\"rootToGlossInput\"', 'wrong', '\u{8fdc}\u{79bb}', '\"incorrect\"', 100, '2026-04-30T10:01:00Z')",
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
        assert_eq!(wrong_words[0]["meanings"][0], "\u{8fdc}\u{79bb}");

        let detail = load_wrong_word_detail_payload_with_bundle(&conn, 1, Some(&bundle_dir))
            .expect("load root affix detail");
        assert_eq!(detail["meanings"][0]["meaningCn"], "\u{8fdc}\u{79bb}");
        assert_eq!(detail["examples"][0]["sentenceEn"], "absorb");
        assert_eq!(detail["examples"][0]["sentenceCn"], "\u{5438}\u{6536}");
        assert_eq!(detail["examples"][1]["sentenceEn"], "abstain");

        fs::remove_dir_all(bundle_dir).expect("cleanup root affix fixture");
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
    fn ai_passage_split_breaks_equal_today_counts_by_total_error_count() {
        let words = (0..12)
            .map(|index| {
                serde_json::json!({
                    "entryId": index + 1,
                    "word": format!("word{index}"),
                    "primaryGloss": format!("meaning {index}"),
                    "partOfSpeech": "n.",
                    "todayWrongCount": 3,
                    "errorCount": index
                })
            })
            .collect::<Vec<_>>();

        let (body_words, other_words) = split_ai_passage_wrong_words(words);
        let body = body_words
            .iter()
            .map(|item| item["word"].as_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>();
        let other = other_words
            .iter()
            .map(|item| item["word"].as_str().unwrap_or_default().to_string())
            .collect::<Vec<_>>();

        assert_eq!(body.len(), AI_PASSAGE_BODY_WORD_LIMIT);
        assert_eq!(body.first().map(String::as_str), Some("word11"));
        assert_eq!(body.last().map(String::as_str), Some("word2"));
        assert_eq!(other, vec!["word1".to_string(), "word0".to_string()]);
    }
    #[test]
    fn ai_passage_split_prioritizes_top_ten_and_appends_other_words() {
        let words = (0..12)
            .map(|index| {
                serde_json::json!({
                    "entryId": index + 1,
                    "word": format!("word{index}"),
                    "primaryGloss": format!("meaning {index}"),
                    "partOfSpeech": "n.",
                    "todayWrongCount": if index == 10 { 4 } else { index % 4 },
                    "errorCount": 100 - index
                })
            })
            .collect::<Vec<_>>();

        let (body_words, other_words) = split_ai_passage_wrong_words(words);

        assert_eq!(body_words.len(), AI_PASSAGE_BODY_WORD_LIMIT);
        assert_eq!(body_words[0]["word"], "word10");
        assert_eq!(other_words.len(), 2);
        let mut blocks = vec![serde_json::json!({"blockType": "paragraph", "text": "body"})];
        append_other_wrong_words_block(&mut blocks, &other_words);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[1]["text"]
            .as_str()
            .expect("other words text")
            .contains("word"));
    }

    #[test]
    fn mastered_entries_are_hidden_from_wrong_words_and_ai_inputs() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-mastered-wrong-filter', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'mastered_word', 'mastered', 'v.', 2.0),
                    (2, 1, 'active_word', 'active', 'v.', 1.0)",
            [],
        )
        .expect("insert entries");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'v.', 'mastered meaning', 0),
                    (2, 'v.', 'active meaning', 0)",
            [],
        )
        .expect("insert meanings");
        conn.execute(
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_mastered_filter', '\"mixedTest\"', 2, '2026-05-05T01:00:00Z', NULL)",
            [],
        )
        .expect("insert session");
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response,
             correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_mastered_filter', 'q_mastered', 1, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-05T01:01:00Z'),
                    ('sess_mastered_filter', 'q_active', 2, '\"enToCnChoice\"', 'A', 'B', '\"incorrect\"', 100, '2026-05-05T01:02:00Z')",
            [],
        )
        .expect("insert wrong results");
        word_storage_core::persistence::mastered_entry_repo::mark_mastered_by_source_id(
            &conn,
            "mastered_word",
            "mastered",
        )
        .expect("mark mastered");

        let wrong_words = load_wrong_word_entries(&conn).expect("load wrong words");
        assert_eq!(wrong_words.len(), 1);
        assert_eq!(wrong_words[0]["word"], "active");

        let ai_inputs =
            load_wrong_word_inputs(&conn, AI_PASSAGE_MAX_WRONG_WORDS).expect("load AI inputs");
        assert_eq!(ai_inputs.len(), 1);
        assert_eq!(ai_inputs[0]["word"], "active");
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
             VALUES (1, 'v.', 'cancel meaning one', 0),
                    (2, 'vt.', 'cancel meaning two', 0)",
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
                "newWordsPerDay": 4,
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
                cn_choice_distractors: Vec::new(),
                en_choice_distractors: Vec::new(),
            }],
            distractor_payloads: Vec::new(),
            question_type_weights: Vec::new(),
        };
        let start = word_app_core::start_study_session(&conn, request).expect("start session");
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

        let completed = build_authoritative_today_home_state(&conn).expect("build completed today");
        let completed_snapshot = completed.today_snapshot.expect("completed snapshot");
        assert_eq!(completed_snapshot.new_words_completed, 4);
    }

    #[test]
    fn incomplete_study_submit_enqueues_shared_event_without_heavy_legacy_snapshots() {
        let conn = Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute("INSERT INTO source_versions (id, source_commit, status) VALUES (1, 'submit-sync-test', 'ready')", []).expect("source");
        conn.execute("INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma) VALUES (1, 1, 'KaoYan_3_1', 'process', 'process')", []).expect("entry");
        conn.execute("INSERT INTO wordbooks (id, code, name, source_book_id, source_version_id, total_entries) VALUES (1, 'kaoyan', 'Kaoyan', 'kaoyan', 1, 1)", []).expect("book");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book) VALUES (1, 1, 1)",
            [],
        )
        .expect("book entry");
        conn.execute("INSERT INTO study_sessions (session_id, mode, total_words, wordbook_id, started_at, completed_at) VALUES ('old-session', 'review', 1, 1, '2026-08-20T00:00:00Z', '2026-08-20T00:01:00Z'), ('active-session', 'highFrequency', 2, 1, '2026-08-24T00:00:00Z', NULL)", []).expect("session");
        conn.execute("INSERT INTO study_results (session_id, question_id, entry_id, question_type, user_response, correct_answer, outcome, response_time_ms, answered_at, hint_used) VALUES ('old-session', 'q-old', 1, 'enToCnInput', '过程', '过程；进程', 'correct', 900, '2026-08-20T00:00:30Z', 0), ('active-session', 'q-1', 1, 'enToCnInput', '过程', '过程；进程', 'correct', 1200, '2026-08-24T00:00:30Z', 0)", []).expect("result");

        enqueue_study_sync_snapshots_after_submit(
            &conn,
            Some(("q-1", "2026-08-24T00:00:30Z")),
            false,
        );

        let pending = word_storage_core::persistence::sync_repo::list_pending_outbox_items(&conn)
            .expect("pending outbox");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].domain, "study_events");
        let payload: serde_json::Value =
            serde_json::from_str(&pending[0].payload_json).expect("payload");
        assert_eq!(
            payload
                .get("events")
                .and_then(|value| value.as_array())
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            payload
                .pointer("/events/0/payloadJson/mode")
                .and_then(|value| value.as_str()),
            Some("highFrequency")
        );
    }

    #[test]
    fn study_session_actions_skip_schema_application_on_the_submit_path() {
        assert!(!should_apply_schema_on_study_session_action());
    }

    #[test]
    fn study_dispute_accept_does_not_enqueue_heavy_sync_snapshots() {
        assert!(!should_enqueue_study_sync_snapshots_after_dispute_accept());
    }

    #[test]
    fn hydrate_start_session_overfetches_invalid_study_payloads_to_match_target() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        word_app_core::clear_all_active_sessions();
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-study-overfetch-invalid', 'ready')",
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
             VALUES (1, 1, 'invalid_a_dot', 'a.', 'n.', 3.0),
                    (2, 1, 'valid_alpha', 'alpha', 'n.', 2.0),
                    (3, 1, 'valid_beta', 'beta', 'n.', 1.0)",
            [],
        )
        .expect("insert entries");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'n.', 'invalid meaning', 0),
                    (2, 'n.', 'alpha meaning', 0),
                    (3, 'n.', 'beta meaning', 0)",
            [],
        )
        .expect("insert meanings");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1), (3, 2, 2), (3, 3, 3)",
            [],
        )
        .expect("insert wordbook entries");
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
                "newWordsPerDay": 0,
                "reviewWordsPerDay": 0,
                "mixedTestPerDay": 2,
                "wrongWordTestPerDay": 0,
                "rootAffixPerDay": 0
            }),
        )
        .expect("set today plan");

        let hydrated = hydrate_start_session_request(
            &conn,
            word_storage_core::models::StartSessionRequest {
                mode: word_storage_core::models::SessionMode::MixedTest,
                wordbook_id: None,
                entry_source_ids: Vec::new(),
                entry_payloads: Vec::new(),
                distractor_payloads: Vec::new(),
                question_type_weights: vec![word_storage_core::models::QuestionTypeWeight {
                    question_type: word_storage_core::models::QuestionType::EnToCnChoice,
                    weight: 100,
                }],
            },
            None,
        )
        .expect("hydrate start request");

        assert_eq!(hydrated.entry_payloads.len(), 2);
        assert_eq!(hydrated.entry_source_ids.len(), 2);
        assert!(hydrated
            .entry_payloads
            .iter()
            .all(|payload| payload.word != "a."));
    }

    #[test]
    fn today_active_session_completion_does_not_rewrite_plan_target() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let today = today_date_string();
        let started_at = chrono::Local::now().to_rfc3339();
        let questions = (0..33)
            .map(|index| {
                serde_json::json!({
                    "questionId": format!("q{index}"),
                    "word": format!("word{index}")
                })
            })
            .collect::<Vec<_>>();
        let snapshot = serde_json::json!({
            "session": {
                "sessionId": "sess_active_mixed_target",
                "startedAt": started_at
            },
            "questions": questions,
            "results": [
                {"questionId": "q0"},
                {"questionId": "q0"},
                {"questionId": "q1"}
            ]
        });
        set_json_setting(&conn, "active_study_session_mixedTest", &snapshot)
            .expect("store active mixed snapshot");

        let targets = align_today_targets_to_active_sessions(
            &conn,
            TodayTargetSeed {
                new_words_target: Some(0),
                new_words_base_target: Some(0),
                new_words_carryover_target: Some(0),
                review_words_target: Some(0),
                review_words_base_target: Some(0),
                review_words_carryover_target: Some(0),
                mixed_test_target: Some(34),
                mixed_test_base_target: Some(34),
                mixed_test_carryover_target: Some(0),
                wrong_word_test_target: Some(0),
                wrong_word_test_base_target: Some(0),
                wrong_word_test_carryover_target: Some(0),
                high_frequency_target: Some(0),
                high_frequency_base_target: Some(0),
                high_frequency_carryover_target: Some(0),
                root_affix_target: Some(0),
                root_affix_base_target: Some(0),
                root_affix_carryover_target: Some(0),
            },
            &today,
        )
        .expect("align active targets");

        assert_eq!(targets.mixed_test_target, Some(34));
        assert_eq!(
            snapshot_answered_question_count(&snapshot),
            Some(2),
            "active progress should dedupe duplicate retry rows"
        );
    }

    #[test]
    fn today_completed_completions_dedupe_without_rewriting_plan_target() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let today = today_date_string();
        let completed_at = chrono::Local::now().to_rfc3339();

        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'test-today-target-dedupe', 'ready')",
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
            "INSERT INTO study_sessions (session_id, mode, total_words, started_at, completed_at)
             VALUES ('sess_completed_mixed_target', '\"mixedTest\"', 34, ?1, ?1)",
            [&completed_at],
        )
        .expect("insert completed session");

        for index in 0..33 {
            conn.execute(
                "INSERT INTO study_results (session_id, question_id, entry_id, question_type,
                 user_response, correct_answer, outcome, response_time_ms, answered_at)
                 VALUES ('sess_completed_mixed_target', ?1, 1, '\"enToCnChoice\"',
                 'A', 'A', '\"correct\"', 100, ?2)",
                rusqlite::params![format!("q{index}"), completed_at],
            )
            .expect("insert completed result");
        }
        conn.execute(
            "INSERT INTO study_results (session_id, question_id, entry_id, question_type,
             user_response, correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('sess_completed_mixed_target', 'q0', 1, '\"enToCnChoice\"',
             'A', 'A', '\"correct\"', 100, ?1)",
            [&completed_at],
        )
        .expect("insert duplicate retry result");

        let targets = align_today_targets_to_completed_sessions(
            &conn,
            TodayTargetSeed {
                new_words_target: Some(0),
                new_words_base_target: Some(0),
                new_words_carryover_target: Some(0),
                review_words_target: Some(0),
                review_words_base_target: Some(0),
                review_words_carryover_target: Some(0),
                mixed_test_target: Some(34),
                mixed_test_base_target: Some(34),
                mixed_test_carryover_target: Some(0),
                wrong_word_test_target: Some(0),
                wrong_word_test_base_target: Some(0),
                wrong_word_test_carryover_target: Some(0),
                high_frequency_target: Some(0),
                high_frequency_base_target: Some(0),
                high_frequency_carryover_target: Some(0),
                root_affix_target: Some(0),
                root_affix_base_target: Some(0),
                root_affix_carryover_target: Some(0),
            },
            &today,
        )
        .expect("align completed targets");
        let completions = load_today_completion_seed(&conn, &today).expect("load completions");

        assert_eq!(targets.mixed_test_target, Some(34));
        assert_eq!(completions.mixed_test_completed, 33);
    }

    #[test]
    fn today_available_pool_alignment_does_not_rewrite_plan_targets() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let plan_value = serde_json::json!({
            "newWordsPerDay": 32,
            "reviewWordsPerDay": 28,
            "mixedTestPerDay": 34,
            "wrongWordTestPerDay": 45,
            "highFrequencyPerDay": 85,
            "rootAffixPerDay": 4
        });

        let targets = align_today_targets_to_available_pools(
            &conn,
            TodayTargetSeed {
                new_words_target: Some(32),
                new_words_base_target: Some(32),
                new_words_carryover_target: Some(0),
                review_words_target: Some(28),
                review_words_base_target: Some(28),
                review_words_carryover_target: Some(0),
                mixed_test_target: Some(34),
                mixed_test_base_target: Some(34),
                mixed_test_carryover_target: Some(0),
                wrong_word_test_target: Some(45),
                wrong_word_test_base_target: Some(45),
                wrong_word_test_carryover_target: Some(0),
                high_frequency_target: Some(85),
                high_frequency_base_target: Some(85),
                high_frequency_carryover_target: Some(0),
                root_affix_target: Some(4),
                root_affix_base_target: Some(4),
                root_affix_carryover_target: Some(0),
            },
            &plan_value,
        )
        .expect("align available pools");

        assert_eq!(targets.new_words_target, Some(32));
        assert_eq!(targets.review_words_target, Some(28));
        assert_eq!(targets.mixed_test_target, Some(34));
        assert_eq!(targets.wrong_word_test_target, Some(45));
        assert_eq!(targets.high_frequency_target, Some(85));
        assert_eq!(targets.root_affix_target, Some(4));
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
    fn root_affix_medical_selection_has_reliable_scoped_cards() {
        let bundle_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/mobile/android/app/src/main/assets");

        let payloads = super::load_root_affix_payloads_for_active_wordbooks(&bundle_dir, &[4], 5)
            .expect("load medical root/affix payloads");

        assert!(!payloads.is_empty());
        assert!(
            payloads
                .iter()
                .all(|payload| payload.source_id.starts_with("root_affix_medical_")),
            "medical root/affix mode should stay in medical scope"
        );
        assert!(payloads.iter().all(|payload| {
            payload
                .meanings
                .iter()
                .any(|meaning| super::contains_han(meaning))
                && payload.example_sentence.is_some()
                && payload.example_translation.is_some()
        }));
    }

    #[test]
    fn root_affix_medical_selection_uses_builtin_cards_when_asset_missing() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("word-medical-root-affix-test-{nonce}"));
        fs::create_dir_all(&bundle_dir).expect("create empty bundle dir");

        let payloads = super::load_root_affix_payloads_for_active_wordbooks(&bundle_dir, &[4], 5)
            .expect("load fallback medical root/affix payloads");

        assert!(!payloads.is_empty());
        assert!(payloads
            .iter()
            .all(|payload| payload.source_id.starts_with("root_affix_medical_")));

        fs::remove_dir_all(bundle_dir).expect("cleanup temp bundle dir");
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
            "cardio 闂傚倸鍊搁崐鎼佸磹閹间讲鈧箓顢楅崟顐わ紱闂佸憡娲﹂崐瀣洪鍕幯冾熆鐠虹尨鍔熼柡灞界墦濮婅櫣鎲撮崟顐㈠Ц濠碘槅鍋勭€氼喚鍒掗崼銉ラ唶闁靛濡囬崢浠嬫偡濠婂喚妯€鐎殿喖鍟胯灒閻炴稈鍓濋弬鈧梻浣虹帛閸旀洟鎮洪妸褌鐒婂ù鐓庣摠閻撶姷鎲搁悧鍫濈闁伙絿鍎ら妵鍕閿涘嫬鈷屽Δ鐘靛仜椤戝骞冮埡鍛疀濞达絽婀辩粙鎰版⒒閸屾瑦绁伴柣顭戝亰瀹曘劑顢欑紒銏℃嵍rdiology(闂傚倸鍊搁崐鎼佸磹閹间讲鈧箓顢楅崟顐わ紱闂佸憡娲﹂崐瀣洪鍕幯冾熆鐠虹尨鍔熼柡灞界墦濮婅櫣鎲撮崟顐㈠Ц濠碘槅鍋勭€氼喚鍒掗崼銉ラ唶婵犮垺绻傜紞濠囧箖閳╁啯鍎熼柨婵嗘閸犳牠姊绘担渚劸濡ょ姵鎮傝棟闁汇垻顭堥拑鐔哥箾閹寸們姘ｉ崼銉︾厱婵°倕鍟禒婊堟倵濞戝磭绉慨?\n",
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
                "trans":[{"tranCn":"闂傚倸鍊搁崐鎼佸磹閹间礁纾归柣鎴ｅГ閸婂潡鏌ㄩ弴姘舵濞存粌缍婇弻娑㈠箛閸忓摜鏁栭梺娲诲幗閹瑰洭寮婚敐澶婎潊闁靛繆鏅濋崝鎼佹⒑閽樺鏆熼柛鐘查叄閸╃偤骞嬮敂钘変汗闂佸湱绮濠氬磻閹剧粯鍋ㄧ紒瀣硶閸濇绻涚€电孝妞ゆ垵妫濋幃锟犲礃椤忓懎鏋戝┑鐘诧工閻楀棛绮堥崼鐔虹闁糕剝锚閻忋儳鈧?}],
                "remMethod":{"val":"re(闂傚倸鍊搁崐鎼佸磹閹间礁纾归柣鎴ｅГ閸婂潡鏌ㄩ弴姘舵濞存粌缍婇弻娑㈠箛閸忓摜鏁栭梺娲诲幗閹瑰洭寮婚敐澶婎潊闁靛繆鏅濋崝鎼佹⒑? + active(婵犵數濮烽弫鍛婃叏閻戣棄鏋侀柟闂寸绾剧粯绻涢幋鐑囦緵闁搞倖娲熼幃瑙勬姜閹殿喚协闂佹寧绻傞ˇ顖滅不閹惰姤鐓涢柛鎰╁妼閳ь剙顭烽幃? -> 闂傚倸鍊搁崐鎼佸磹閹间礁纾归柣鎴ｅГ閸婂潡鏌ㄩ弴姘舵濞存粌缍婇弻娑㈠箛閸忓摜鏁栭梺娲诲幗閹瑰洭寮婚敐澶婎潊闁靛繆鏅濋崝鎼佹⒑閽樺鏆熼柛鐘查叄閸╃偤骞嬮敂钘変汗闂佸湱绮濠氬磻閹剧粯鍋ㄧ紒瀣硶閸濇绻涚€电孝妞ゆ垵妫濋幃锟犲礃椤忓懎鏋戝┑鐘诧工閻楀棛绮堥崼鐔虹闁糕剝锚閻忋儳鈧?}
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
            "today_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select today wordbook");
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
                question_type_weights: Vec::new(),
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
            serde_json::json!([
                {
                    "headWord": "guarantee",
                    "content": {"word": {"wordHead": "guarantee", "content": {
                        "trans": [{"tranCn": "\u{4fdd}\u{8bc1}"}],
                        "remMethod": {"val": "guar(\u{4fdd}\u{62a4}) + antee -> guarantee"}
                    }}}
                },
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
            ])
            .to_string(),
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
        assert!(
            payloads
                .iter()
                .all(|payload| payload.source_id != "root_affix_shared_ear"),
            "tail fragments like ear should be filtered out"
        );
        assert!(
            payloads
                .iter()
                .all(|payload| payload.source_id != "root_affix_shared_exe"),
            "mnemonic-only fragments like exe should be filtered out"
        );
        let re = payloads
            .iter()
            .find(|payload| payload.source_id == "root_affix_shared_re")
            .expect("multi-example re root should remain");
        assert_eq!(re.example_sentence.as_deref(), Some("reactivate, rebuild"));
        assert_eq!(
            re.example_translation.as_deref(),
            Some("\u{91cd}\u{65b0}\u{6fc0}\u{6d3b}, \u{91cd}\u{5efa}")
        );

        fs::remove_dir_all(bundle_dir).expect("cleanup temp bundle dir");
    }

    #[test]
    fn root_affix_shared_cards_reject_prefix_lookalikes_and_duplicate_meanings() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("word-root-affix-lookalike-test-{nonce}"));
        let book_dir = bundle_dir.join("seed-vocab").join("book");
        fs::create_dir_all(&book_dir).expect("create root affix fixture dir");
        fs::write(
            book_dir.join("KaoYan_3.json"),
            serde_json::json!([
                {
                    "headWord": "cabin",
                    "content": {"word": {"wordHead": "cabin", "content": {
                        "trans": [{"tranCn": "\u{5c0f}\u{5c4b}"}],
                        "remMethod": {"val": "cab(\u{51fa}\u{79df}\u{8f66}) + in -> cabin"}
                    }}}
                },
                {
                    "headWord": "cabinet",
                    "content": {"word": {"wordHead": "cabinet", "content": {
                        "trans": [{"tranCn": "\u{67dc}"}],
                        "remMethod": {"val": "cab(\u{51fa}\u{79df}\u{8f66}) + inet -> cabinet"}
                    }}}
                },
                {
                    "headWord": "cabbage",
                    "content": {"word": {"wordHead": "cabbage", "content": {
                        "trans": [{"tranCn": "\u{6d0b}\u{767d}\u{83dc}"}],
                        "remMethod": {"val": "cab(\u{51fa}\u{79df}\u{8f66}) + bage -> cabbage"}
                    }}}
                },
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
                },
                {
                    "headWord": "combine",
                    "content": {"word": {"wordHead": "combine", "content": {
                        "trans": [{"tranCn": "\u{7ed3}\u{5408}"}],
                        "remMethod": {"val": "com(\u{5171}\u{540c}) + bine -> combine"}
                    }}}
                },
                {
                    "headWord": "compose",
                    "content": {"word": {"wordHead": "compose", "content": {
                        "trans": [{"tranCn": "\u{7ec4}\u{6210}"}],
                        "remMethod": {"val": "com(\u{5171}\u{540c}) + pose -> compose"}
                    }}}
                },
                {
                    "headWord": "synonym",
                    "content": {"word": {"wordHead": "synonym", "content": {
                        "trans": [{"tranCn": "\u{540c}\u{4e49}\u{8bcd}"}],
                        "remMethod": {"val": "syn(\u{5171}\u{540c}) + onym -> synonym"}
                    }}}
                },
                {
                    "headWord": "syntax",
                    "content": {"word": {"wordHead": "syntax", "content": {
                        "trans": [{"tranCn": "\u{8bed}\u{6cd5}"}],
                        "remMethod": {"val": "syn(\u{5171}\u{540c}) + tax -> syntax"}
                    }}}
                }
            ])
            .to_string(),
        )
        .expect("write root affix fixture");

        let payloads = super::load_root_affix_payloads_for_active_wordbooks_on_date(
            &bundle_dir,
            &[3],
            10,
            "2026-05-19",
        )
        .expect("load root affix payloads");
        let source_ids = payloads
            .iter()
            .map(|payload| payload.source_id.as_str())
            .collect::<Vec<_>>();

        assert!(
            !source_ids.contains(&"root_affix_shared_cab"),
            "cab taxi mnemonics must not become root-affix cards"
        );
        assert!(
            source_ids.contains(&"root_affix_shared_re"),
            "known shared prefix re should remain"
        );
        let shared_common_count = payloads
            .iter()
            .filter(|payload| {
                payload
                    .meanings
                    .iter()
                    .any(|meaning| meaning == "\u{5171}\u{540c}")
            })
            .count();
        assert_eq!(
            shared_common_count, 1,
            "duplicate same-meaning root cards should not appear in one session"
        );

        fs::remove_dir_all(bundle_dir).expect("cleanup temp bundle dir");
    }

    #[test]
    fn root_affix_shared_cards_promote_common_longer_prefixes() {
        let _cards = super::parse_shared_root_affix_cards(
            "distinguish",
            "distinguish meaning",
            "di(闂傚倷绀侀幉锛勬暜閹烘嚦娑樜旈崘鈺婃綗? + stinguish -> distinguish",
        );

        let _cards = super::parse_shared_root_affix_cards(
            "distinguish",
            "distinguish meaning",
            "di(分离) + stinguish -> distinguish",
        );

        let cards = super::parse_shared_root_affix_cards(
            "distinguish",
            "distinguish meaning",
            "di(apart) + stinguish -> distinguish",
        );

        assert!(cards
            .iter()
            .any(|card| card.id == "root_affix_shared_dis" && card.form == "dis-"));
        assert!(cards.iter().all(|card| card.id != "root_affix_shared_di"));
    }

    #[test]
    fn question_preps_are_generated_from_same_wordbook_and_part_of_speech() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'question-preps-test', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (10, 'BookA', 'BookA', 1, 4, 1), (20, 'BookB', 'BookB', 1, 1, 1)",
            [],
        )
        .expect("insert wordbooks");

        let entries = [
            (1, "target", "target", "n.", "core base", 10, 1),
            (2, "same_book_one", "basis", "n.", "base principle", 10, 2),
            (3, "same_book_two", "fund", "n.", "money fund", 10, 3),
            (4, "same_book_verb", "found", "v.", "build create", 10, 4),
            (5, "other_book", "capital", "n.", "money capital", 20, 1),
        ];
        for (id, key, word, pos, meaning, wordbook_id, rank) in entries {
            conn.execute(
                "INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma, part_of_speech)
                 VALUES (?1, 1, ?2, ?3, ?3, ?4)",
                rusqlite::params![id, key, word, pos],
            )
            .expect("insert entry");
            conn.execute(
                "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
                 VALUES (?1, ?2, ?3, 0)",
                rusqlite::params![id, pos, meaning],
            )
            .expect("insert meaning");
            conn.execute(
                "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![wordbook_id, id, rank],
            )
            .expect("insert wordbook entry");
        }

        rebuild_seed_question_preps(&conn).expect("rebuild question preps");
        let (cn_json, en_json): (String, String) = conn
            .query_row(
                "SELECT cn_choice_distractors_json, en_choice_distractors_json
                 FROM entry_question_preps
                 WHERE entry_id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load target preps");
        let cn = serde_json::from_str::<Vec<String>>(&cn_json).expect("decode cn preps");
        let en = serde_json::from_str::<Vec<String>>(&en_json).expect("decode en preps");

        assert!(cn.iter().any(|text| text == "base principle"));
        assert!(cn.iter().any(|text| text == "money fund"));
        assert!(!cn.iter().any(|text| text == "build create"));
        assert!(!cn.iter().any(|text| text == "money capital"));
        assert!(en.contains(&"basis".to_string()));
        assert!(en.contains(&"fund".to_string()));
        assert!(!en.contains(&"found".to_string()));
        assert!(!en.contains(&"capital".to_string()));
    }

    #[test]
    fn seed_maintenance_replaces_existing_generated_preps_with_bundled_safe_preps() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'safe-question-preps-test', 'ready')",
            [],
        )
        .expect("insert source version");
        conn.execute(
            "INSERT INTO wordbooks (id, code, name, source_version_id, total_entries, is_active)
             VALUES (1, 'cet4', 'CET-4', 1, 1, 1)",
            [],
        )
        .expect("insert wordbook");
        conn.execute(
            "INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma, part_of_speech)
             VALUES (1, 1, 'target', 'curb', 'curb', 'vt')",
            [],
        )
        .expect("insert entry");
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (1, 'vt', '\u{63a7}\u{5236}\u{ff1b}\u{7ea6}\u{675f}', 0)",
            [],
        )
        .expect("insert meaning");
        conn.execute(
            "INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (1, 1, 1)",
            [],
        )
        .expect("insert wordbook entry");
        conn.execute(
            "INSERT INTO entry_question_preps
                (entry_id, cn_choice_distractors_json, en_choice_distractors_json)
             VALUES (1, '[\"\u{6291}\u{5236}\u{ff1b}\u{514b}\u{5236}\"]', '[\"refrain\"]')",
            [],
        )
        .expect("insert unsafe prep");

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("word-safe-preps-{nonce}"));
        let book_dir = bundle_dir.join("seed-vocab").join("book");
        fs::create_dir_all(&book_dir).expect("create temp book dir");
        fs::write(
            book_dir.join("CET4_3.json"),
            serde_json::json!([{
                "headWord": "curb",
                "displayWord": "curb",
                "cnChoiceDistractors": ["\u{5236}\u{9020}\u{ff1b}\u{52a0}\u{5de5}", "\u{6350}\u{732e}\u{ff1b}\u{6350}\u{6b3e}", "\u{4f7f}\u{5bb3}\u{6015}"],
                "enChoiceDistractors": ["manufacture", "donate", "frighten"],
                "content": {"word": {"content": {"trans": [{"pos": "vt", "tranCn": "\u{63a7}\u{5236}\u{ff1b}\u{7ea6}\u{675f}"}]}}}
            }])
            .to_string(),
        )
        .expect("write seed fixture");

        ensure_seed_vocabulary_available_for_today(&conn, &bundle_dir)
            .expect("run safe question prep maintenance from Today startup");
        let (cn_json, en_json): (String, String) = conn
            .query_row(
                "SELECT cn_choice_distractors_json, en_choice_distractors_json
                 FROM entry_question_preps WHERE entry_id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("load refreshed prep");
        let cn: Vec<String> = serde_json::from_str(&cn_json).expect("decode cn preps");
        let en: Vec<String> = serde_json::from_str(&en_json).expect("decode en preps");
        assert_eq!(
            cn,
            vec![
                "\u{5236}\u{9020}\u{ff1b}\u{52a0}\u{5de5}",
                "\u{6350}\u{732e}\u{ff1b}\u{6350}\u{6b3e}",
                "\u{4f7f}\u{5bb3}\u{6015}",
            ]
        );
        assert_eq!(en, vec!["manufacture", "donate", "frighten"]);
        assert!(get_json_setting(
            &conn,
            "seed_vocabulary_maintenance_v6_exam_frequency",
            &serde_json::Value::Bool(false),
        )
        .expect("load maintenance marker")
        .as_bool()
        .unwrap_or(false));
        assert!(get_json_setting(
            &conn,
            "seed_vocabulary_maintenance_v7_real_exam_examples",
            &serde_json::Value::Bool(false),
        )
        .expect("load real-exam maintenance marker")
        .as_bool()
        .unwrap_or(false));
        fs::remove_dir_all(bundle_dir).expect("cleanup temp bundle dir");
    }
    #[test]
    fn entry_payload_prefers_real_exam_source_example() {
        let examples = vec![
            serde_json::json!({
                "sentenceEn": "A generic dictionary sentence.",
                "sentenceCn": "段；片；部分"
            }),
            serde_json::json!({
                "sentenceEn": "Demand from the food service segment grew.",
                "sentenceCn": "真题来源：KAOYAN-ENGLISH-1 / Kaoyan English I 2010"
            }),
        ];
        let meanings = vec![word_storage_core::models::StartSessionMeaningPayload {
            pos: "n".to_string(),
            meaning_cn: "段；片；部分".to_string(),
            meaning_en: None,
        }];

        let selected = super::select_entry_payload_example(&examples, &meanings, Some("n"))
            .expect("selected example");

        assert_eq!(
            selected["sentenceEn"],
            "Demand from the food service segment grew."
        );
    }

    #[test]
    fn real_exam_example_maintenance_updates_existing_seed_entries() {
        let conn = Connection::open_in_memory().expect("open database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        conn.execute_batch(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'seed-example-refresh', 'ready');
             INSERT INTO wordbooks (id, code, name, category, source_version_id)
             VALUES (3, 'kaoyan', 'KaoYan', 'exam', 1);
             INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma)
             VALUES (7, 1, 'segment-source', 'segment', 'segment');
             INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 7, 1);
             INSERT INTO entry_examples (entry_id, sentence_en, sentence_cn, sort_order)
             VALUES
               (7, 'A generic segment example.', '普通例句', 0),
               (7, 'An outdated exam example.', '真题来源：OLD', 1);",
        )
        .expect("seed existing entry");
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let book_dir = env::temp_dir().join(format!("word-real-exam-refresh-{nonce}"));
        fs::create_dir_all(&book_dir).expect("create book dir");
        fs::write(
            book_dir.join("KaoYan_3.json"),
            serde_json::json!([{
                "headWord": "segment",
                "content": {"word": {"content": {"realExamSentence": {"sentences": [{
                    "sContent": "Demand from the food service segment grew.",
                    "sCn": "真题来源：KAOYAN-ENGLISH-1 / Kaoyan English I 2010"
                }]}}}}
            }])
            .to_string(),
        )
        .expect("write book fixture");

        super::refresh_seed_real_exam_examples(&conn, &book_dir).expect("refresh examples");

        let examples = super::load_entry_examples(&conn, 7).expect("load examples");
        assert!(examples
            .iter()
            .any(|example| { example["sentenceEn"] == "A generic segment example." }));
        assert!(examples.iter().any(|example| {
            example["sentenceEn"] == "Demand from the food service segment grew."
        }));
        assert!(!examples
            .iter()
            .any(|example| { example["sentenceEn"] == "An outdated exam example." }));
        fs::remove_dir_all(book_dir).expect("cleanup book dir");
    }

    #[test]
    fn load_entry_payload_merges_user_disputed_meanings() {
        let conn = rusqlite::Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute(
            "INSERT INTO source_versions (source_commit, status) VALUES ('payload-dispute-v1', 'ready')",
            [],
        )
        .expect("insert source version");
        let source_version_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO entries (source_version_id, source_entry_key, word, lemma)
             VALUES (?1, 'payload_alpha', 'alpha', 'alpha')",
            [source_version_id],
        )
        .expect("insert entry");
        let entry_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES (?1, 'n', '\u{963f}\u{5c14}\u{6cd5}', 0)",
            [entry_id],
        )
        .expect("insert meaning");
        word_storage_core::persistence::user_accepted_meaning_repo::save_user_dispute(
            &conn,
            "payload_alpha",
            "q1",
            "enToCnInput",
            "\u{5b57}\u{6bcd}\u{9996}\u{4f4d}",
        )
        .expect("save dispute");

        let payload = super::load_entry_payload(&conn, entry_id).expect("load payload");

        assert!(payload
            .meanings
            .contains(&"\u{5b57}\u{6bcd}\u{9996}\u{4f4d}".to_string()));
        assert!(payload.meaning_details.iter().any(|meaning| {
            meaning.pos == "user_dispute"
                && meaning.meaning_cn == "\u{5b57}\u{6bcd}\u{9996}\u{4f4d}"
        }));
    }

    #[test]
    fn wrong_word_payloads_include_user_disputed_meanings() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let entry_id = seed_basic_entry(&conn, "appeal", "\u{539f}\u{59cb}\u{91ca}\u{4e49}");
        conn.execute(
            "INSERT INTO study_results
             (session_id, question_id, entry_id, question_type, user_response,
              correct_answer, outcome, response_time_ms, answered_at)
             VALUES ('s1', 'q1', ?1, 'enToCnInput', '\u{65e7}\u{7b54}\u{6848}', '\u{539f}\u{59cb}\u{91ca}\u{4e49}', 'incorrect', 1, '2026-05-04T00:00:00Z')",
            [entry_id],
        )
        .expect("insert result");
        word_storage_core::persistence::user_accepted_meaning_repo::save_user_dispute(
            &conn,
            "test_appeal",
            "q1",
            "enToCnInput",
            "\u{65b0}\u{589e}\u{91ca}\u{4e49}",
        )
        .expect("save dispute");

        let wrong_words = load_wrong_word_entries(&conn).expect("load wrong words");
        let meanings = wrong_words[0]["meanings"].as_array().expect("meanings");
        assert!(meanings
            .iter()
            .any(|meaning| meaning == "\u{65b0}\u{589e}\u{91ca}\u{4e49}"));

        let detail =
            load_wrong_word_detail_payload_with_bundle(&conn, entry_id, None).expect("detail");
        let detail_meanings = detail["meanings"].as_array().expect("detail meanings");
        assert!(detail_meanings.iter().any(|meaning| {
            meaning["pos"] == "user_dispute"
                && meaning["meaningCn"] == "\u{65b0}\u{589e}\u{91ca}\u{4e49}"
        }));
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
            "today_wordbooks_json",
            &serde_json::json!({"3": true}),
        )
        .expect("select today wordbook");
        set_json_setting(
            &conn,
            "today_plan_json",
            &serde_json::json!({
                "newWordsPerDay": 4,
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
                question_type_weights: Vec::new(),
            },
            None,
        )
        .expect("hydrate new word restore");

        assert_eq!(hydrated.entry_source_ids, vec!["real_entry"]);
        assert_eq!(hydrated.entry_payloads[0].word, "alpha");
    }

    #[test]
    fn reward_image_upload_entitlement_grants_once_per_five_day_milestone() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");

        let initial = load_reward_image_upload_entitlement(&conn).expect("load initial");
        assert_eq!(initial["availableUploads"], 0);
        assert_eq!(initial["nextMilestoneStreakDays"], 5);

        conn.execute(
            "INSERT INTO reward_image_upload_entitlements (
                owner_key,
                available_uploads,
                last_granted_streak_milestone,
                updated_at
            )
            VALUES ('local', 1, 1, datetime('now'))",
            [],
        )
        .expect("seed first milestone");

        let repeated_five_day =
            refresh_reward_image_entitlement_with_connection(&conn, 5, 3).expect("refresh 5d");
        assert_eq!(repeated_five_day["availableUploads"], 1);
        assert_eq!(repeated_five_day["lastGrantedStreakMilestone"], 1);

        let ten_day =
            refresh_reward_image_entitlement_with_connection(&conn, 10, 3).expect("refresh 10d");
        assert_eq!(ten_day["availableUploads"], 2);
        assert_eq!(ten_day["lastGrantedStreakMilestone"], 2);
        assert_eq!(ten_day["nextMilestoneStreakDays"], 15);

        let repeated_ten_day =
            refresh_reward_image_entitlement_with_connection(&conn, 10, 3).expect("repeat 10d");
        assert_eq!(repeated_ten_day["availableUploads"], 2);
        assert_eq!(repeated_ten_day["lastGrantedStreakMilestone"], 2);
    }

    #[test]
    fn reward_image_public_list_hides_pending_and_rejected_images() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");

        let pending = create_reward_image_upload_with_connection(
            &conn,
            "/tmp/pending.jpg",
            "image/jpeg",
            "pending.jpg",
        )
        .expect("create pending");
        let approved = create_reward_image_upload_with_connection(
            &conn,
            "/tmp/approved.jpg",
            "image/jpeg",
            "approved.jpg",
        )
        .expect("create approved");
        let rejected = create_reward_image_upload_with_connection(
            &conn,
            "/tmp/rejected.jpg",
            "image/jpeg",
            "rejected.jpg",
        )
        .expect("create rejected");

        moderate_reward_image_with_connection(
            &conn,
            approved["id"].as_i64().expect("approved id"),
            "approved",
            "",
        )
        .expect("approve image");
        moderate_reward_image_with_connection(
            &conn,
            rejected["id"].as_i64().expect("rejected id"),
            "rejected",
            "unsafe",
        )
        .expect("reject image");

        let public =
            list_reward_images_with_connection(&conn, true, "2026-05-04").expect("list public");
        let images = public["images"].as_array().expect("images array");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0]["id"], approved["id"]);
        assert_ne!(images[0]["id"], pending["id"]);
        assert_ne!(images[0]["id"], rejected["id"]);
    }

    #[test]
    fn reward_image_vote_is_unique_per_voter_and_week() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let image = create_reward_image_upload_with_connection(
            &conn,
            "/tmp/vote.jpg",
            "image/jpeg",
            "vote.jpg",
        )
        .expect("create image");
        let image_id = image["id"].as_i64().expect("image id");
        moderate_reward_image_with_connection(&conn, image_id, "approved", "")
            .expect("approve image");

        let first = vote_reward_image_with_connection(&conn, image_id, "local", "2026-05-04")
            .expect("first vote");
        let second = vote_reward_image_with_connection(&conn, image_id, "local", "2026-05-04")
            .expect("duplicate vote");
        let next_week = vote_reward_image_with_connection(&conn, image_id, "local", "2026-05-11")
            .expect("next week vote");

        assert_eq!(first["inserted"], true);
        assert_eq!(first["voteCount"], 1);
        assert_eq!(second["inserted"], false);
        assert_eq!(second["voteCount"], 1);
        assert_eq!(next_week["inserted"], true);
        assert_eq!(next_week["voteCount"], 1);
    }

    #[test]
    fn local_leaderboard_demo_seeds_ranked_user_image_and_vote_count() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");

        let seeded = seed_local_leaderboard_demo_with_connection(&conn, None)
            .expect("seed local leaderboard demo");
        let entries = seeded["leaderboard"]["entries"]
            .as_array()
            .expect("leaderboard entries");
        let demo = entries
            .iter()
            .find(|entry| entry["user_id"] == "demo_learner")
            .expect("demo learner row");
        assert_eq!(demo["current_streak_days"], 6);
        assert_eq!(demo["total_questions"], 168);
        assert!(demo["tag_image"].is_object());

        let vote = seeded["vote"].as_object().expect("vote result");
        assert_eq!(vote["inserted"], true);
        assert_eq!(vote["voteCount"], 1);

        let images = seeded["images"]["images"].as_array().expect("image rows");
        let demo_image = images
            .iter()
            .find(|image| image["ownerKey"] == "demo_learner")
            .expect("demo image row");
        assert_eq!(demo_image["voteCount"], 1);
    }

    #[test]
    fn exam_import_output_is_reviewable_and_does_not_retain_raw_media() {
        let raw = serde_json::json!({
            "paper": {
                "schemaVersion": 1,
                "id": "user-cet4-1",
                "exam": "cet4",
                "title": "Imported CET4",
                "year": 2026,
                "source": {},
                "sections": [{
                    "id": "reading",
                    "type": "reading",
                    "title": "Reading",
                    "passage": "A passage.",
                    "questions": [{
                        "id": "q1",
                        "number": 1,
                        "kind": "objective",
                        "stem": "Choose one.",
                        "choices": [{"label": "A", "text": "First"}],
                        "answer": null,
                        "explanation": ""
                    }]
                }]
            },
            "warnings": ["Answer is missing"]
        });
        let normalized =
            super::normalize_exam_import_ai_output(&raw.to_string(), "image", "paper.jpg")
                .expect("normalize exam import");
        let payload: serde_json::Value =
            serde_json::from_str(&normalized).expect("decode normalized draft");

        assert_eq!(payload["rawMediaRetained"], false);
        assert_eq!(payload["provenance"]["sourceName"], "paper.jpg");
        assert_eq!(
            payload["paper"]["sections"][0]["questions"][0]["capabilities"]["autoGradable"],
            false
        );
    }

    #[test]
    fn exam_causal_provider_failure_keeps_a_structured_local_fallback() {
        let result =
            super::normalize_exam_causal_provider_result(Err("provider offline".to_string()));

        assert_eq!(result["eligible"], true);
        assert_eq!(result["providerStatus"], "provider_failure");
        assert!(result["candidates"].as_array().is_some_and(Vec::is_empty));
        assert!(result["limitations"][0]
            .as_str()
            .is_some_and(|value| value.contains("provider offline")));
    }

    #[test]
    fn exam_causal_prompt_requires_a_short_parseable_report() {
        assert!(super::EXAM_CAUSAL_SYSTEM_PROMPT.contains("at most two candidates"));
        assert!(super::EXAM_CAUSAL_SYSTEM_PROMPT.contains("under 350 tokens"));
        assert!(super::EXAM_CAUSAL_SYSTEM_PROMPT.contains("under 80 Simplified Chinese characters"));
    }

    #[test]
    fn exam_causal_response_recovers_unescaped_quotes_inside_a_string_value() {
        let content = r#"{
            "eligible": true,
            "candidates": [{
                "word": "mergers",
                "reasoning": "The "business trend" led to the wrong choice."
            }],
            "limitations": []
        }"#;

        let result = super::normalize_exam_causal_provider_result(Ok(content.to_string()));

        assert_eq!(result["providerStatus"], "completed");
        assert_eq!(result["candidates"][0]["word"], "mergers");
        assert_eq!(
            result["candidates"][0]["reasoning"],
            "The \"business trend\" led to the wrong choice."
        );
    }

    #[test]
    fn exam_causal_prompt_carries_passage_translation_reference_and_marks() {
        let section = word_app_core::services::exam_practice_service::ExamSection {
            id: "text-1".to_string(),
            section_type: "reading".to_string(),
            title: "Text 1".to_string(),
            instructions: String::new(),
            passage: "Mergers changed the market.".to_string(),
            paragraph_translations: vec!["企业合并改变了市场。".to_string()],
            questions: Vec::new(),
        };
        let question = word_app_core::services::exam_practice_service::ExamQuestion {
            id: "q1".to_string(),
            number: 1,
            kind: "objective".to_string(),
            stem: "What changed the market?".to_string(),
            choices: vec![word_app_core::services::exam_practice_service::ExamChoice {
                label: "C".to_string(),
                text: "Mergers".to_string(),
            }],
            answer: Some("C".to_string()),
            explanation: String::new(),
            source: serde_json::json!({}),
            answer_source: None,
            capabilities:
                word_app_core::services::exam_practice_service::ExamQuestionCapabilities {
                    browsable: true,
                    answerable: true,
                    auto_gradable: true,
                    causal_analyzable: true,
                },
        };
        let evidence = vec![serde_json::json!({"word": "mergers", "mark": "unknown"})];

        let prompt = super::build_exam_causal_prompt(&section, &question, Some("A"), &evidence);

        assert_eq!(prompt["passage"], "Mergers changed the market.");
        assert_eq!(prompt["paragraphTranslations"][0], "企业合并改变了市场。");
        assert_eq!(prompt["correctAnswer"], "C");
        assert_eq!(prompt["selectedAnswer"], "A");
        assert_eq!(prompt["markedVocabulary"][0]["word"], "mergers");
    }

    #[test]
    fn section_causal_prompt_sends_shared_article_context_once_for_all_wrong_questions() {
        let section = word_app_core::services::exam_practice_service::ExamSection {
            id: "text-1".to_string(),
            section_type: "reading".to_string(),
            title: "Text 1".to_string(),
            instructions: String::new(),
            passage: "Shared passage.".to_string(),
            paragraph_translations: vec!["Shared translation.".to_string()],
            questions: Vec::new(),
        };
        let questions = vec![
            word_app_core::services::exam_practice_service::ExamQuestion {
                id: "q1".to_string(),
                number: 1,
                kind: "objective".to_string(),
                stem: "First question".to_string(),
                choices: vec![word_app_core::services::exam_practice_service::ExamChoice {
                    label: "B".to_string(),
                    text: "Correct".to_string(),
                }],
                answer: Some("B".to_string()),
                explanation: String::new(),
                source: serde_json::json!({}),
                answer_source: None,
                capabilities:
                    word_app_core::services::exam_practice_service::ExamQuestionCapabilities {
                        browsable: true,
                        answerable: true,
                        auto_gradable: true,
                        causal_analyzable: true,
                    },
            },
            word_app_core::services::exam_practice_service::ExamQuestion {
                id: "q2".to_string(),
                number: 2,
                kind: "objective".to_string(),
                stem: "Second question".to_string(),
                choices: vec![word_app_core::services::exam_practice_service::ExamChoice {
                    label: "C".to_string(),
                    text: "Correct".to_string(),
                }],
                answer: Some("C".to_string()),
                explanation: String::new(),
                source: serde_json::json!({}),
                answer_source: None,
                capabilities:
                    word_app_core::services::exam_practice_service::ExamQuestionCapabilities {
                        browsable: true,
                        answerable: true,
                        auto_gradable: true,
                        causal_analyzable: true,
                    },
            },
        ];
        let selections = vec![
            (questions[0].clone(), "A".to_string()),
            (questions[1].clone(), "A".to_string()),
        ];
        let evidence = vec![serde_json::json!({"word": "shared", "mark": "unknown"})];

        let prompt = super::build_exam_section_causal_prompt(&section, &selections, &evidence);
        let encoded = prompt.to_string();

        assert_eq!(prompt["passage"], "Shared passage.");
        assert_eq!(
            prompt["paragraphTranslations"],
            serde_json::json!(["Shared translation."])
        );
        assert_eq!(prompt["wrongQuestions"].as_array().map(Vec::len), Some(2));
        assert_eq!(prompt["wrongQuestions"][0]["questionId"], "q1");
        assert_eq!(prompt["wrongQuestions"][1]["questionId"], "q2");
        assert_eq!(encoded.matches("Shared passage.").count(), 1);
        assert_eq!(encoded.matches("Shared translation.").count(), 1);
    }

    #[test]
    fn cloze_review_prompt_contains_wrong_and_correct_attempts() {
        let mut section = word_app_core::services::exam_practice_service::ExamSection {
            id: "cloze".to_string(),
            section_type: "cloze".to_string(),
            title: "Cloze".to_string(),
            instructions: String::new(),
            passage: "It was (1) rare but (2) useful.".to_string(),
            paragraph_translations: vec![],
            questions: Vec::new(),
        };
        let question = |id: &str, number: i64, answer: &str| {
            word_app_core::services::exam_practice_service::ExamQuestion {
                id: id.to_string(),
                number,
                kind: "objective".to_string(),
                stem: String::new(),
                choices: vec![word_app_core::services::exam_practice_service::ExamChoice {
                    label: answer.to_string(),
                    text: if id == "q1" { "rare" } else { "useful" }.to_string(),
                }],
                answer: Some(answer.to_string()),
                explanation: String::new(),
                source: serde_json::json!({}),
                answer_source: None,
                capabilities:
                    word_app_core::services::exam_practice_service::ExamQuestionCapabilities {
                        browsable: true,
                        answerable: true,
                        auto_gradable: true,
                        causal_analyzable: true,
                    },
            }
        };
        section.questions = vec![question("q1", 1, "A"), question("q2", 2, "B")];
        let attempts = vec![
            (section.questions[0].clone(), "B".to_string(), false),
            (section.questions[1].clone(), "B".to_string(), true),
        ];
        let evidence = vec![
            serde_json::json!({
                "word": "rare", "mark": "unknown", "meaning": "罕见的",
                "examFrequency": 20, "examRank": 12
            }),
            serde_json::json!({
                "word": "useful", "mark": "familiar", "meaning": "有用的",
                "examFrequency": 15, "examRank": 18
            }),
        ];

        let prompt = super::build_exam_cloze_review_prompt(&section, &attempts, &evidence);

        assert_eq!(prompt["reviewFormat"], "cloze-review-v1");
        assert_eq!(prompt["wrongQuestions"].as_array().map(Vec::len), Some(1));
        assert_eq!(prompt["correctQuestions"].as_array().map(Vec::len), Some(1));
        assert_eq!(prompt["markedVocabulary"][0]["examFrequency"], 20);
    }

    #[test]
    fn reading_review_prompt_contains_report_contract_and_marked_correct_attempts() {
        let mut section = word_app_core::services::exam_practice_service::ExamSection {
            id: "text-2".to_string(),
            section_type: "text2".to_string(),
            title: "Reading Text 2".to_string(),
            instructions: String::new(),
            passage: "The old process changed. A new model is emerging.".to_string(),
            paragraph_translations: vec!["旧流程发生了变化。新模式正在出现。".to_string()],
            questions: Vec::new(),
        };
        let question = |id: &str, number: i64, stem: &str, answer: &str, choice: &str| {
            word_app_core::services::exam_practice_service::ExamQuestion {
                id: id.to_string(),
                number,
                kind: "objective".to_string(),
                stem: stem.to_string(),
                choices: vec![word_app_core::services::exam_practice_service::ExamChoice {
                    label: answer.to_string(),
                    text: choice.to_string(),
                }],
                answer: Some(answer.to_string()),
                explanation: String::new(),
                source: serde_json::json!({}),
                answer_source: None,
                capabilities:
                    word_app_core::services::exam_practice_service::ExamQuestionCapabilities {
                        browsable: true,
                        answerable: true,
                        auto_gradable: true,
                        causal_analyzable: true,
                    },
            }
        };
        section.questions = vec![
            question("q1", 1, "What changed?", "B", "new model"),
            question("q2", 2, "What is emerging?", "A", "emerging"),
        ];
        let attempts = vec![
            (section.questions[0].clone(), "A".to_string(), false),
            (section.questions[1].clone(), "A".to_string(), true),
        ];
        let evidence = vec![serde_json::json!({
            "word": "emerging", "mark": "unknown", "meaning": "出现的"
        })];

        let prompt = super::build_exam_reading_review_prompt(&section, &attempts, &evidence);

        assert_eq!(prompt["reviewFormat"], "reading-review-v1");
        assert_eq!(prompt["wrongQuestions"][0]["stem"], "What changed?");
        assert_eq!(prompt["wrongQuestions"][0]["selectedAnswer"], "A");
        assert_eq!(prompt["wrongQuestions"][0]["correctAnswer"], "B");
        assert_eq!(prompt["correctQuestions"][0]["questionId"], "q2");
    }

    #[test]
    fn cloze_vocabulary_priority_uses_local_frequency_and_rank() {
        let evidence = vec![
            serde_json::json!({"word":"observe","mark":"unknown","meaning":"观察","examFrequency":42,"examRank":8}),
            serde_json::json!({"word":"rare","mark":"familiar","meaning":"罕见的","examFrequency":12,"examRank":30}),
        ];
        let mut result = serde_json::json!({
            "vocabularyPriority": [
                {"word":"rare","priorityReason":"模型理由"},
                {"word":"observe","priorityReason":"高频"}
            ]
        });

        super::attach_cloze_vocabulary_priority(&mut result, &evidence);

        assert_eq!(result["vocabularyPriority"][0]["word"], "observe");
        assert_eq!(result["vocabularyPriority"][0]["examFrequency"], 42);
        assert_eq!(result["vocabularyPriority"][1]["mark"], "familiar");
    }

    #[test]
    fn cloze_vocabulary_priority_displays_derivational_family_total() {
        let evidence = vec![serde_json::json!({
            "word":"observer",
            "mark":"unknown",
            "meaning":"观察者",
            "examFrequency":1,
            "examRank":3000,
            "examFamilyRoot":"observe",
            "examFamilyFrequency":12,
            "examFamilyMembers":{
                "observe":6,
                "observation":3,
                "observer":1,
                "observable":1,
                "observational":1
            }
        })];
        let mut result = serde_json::json!({"vocabularyPriority": []});

        super::attach_cloze_vocabulary_priority(&mut result, &evidence);

        let item = &result["vocabularyPriority"][0];
        assert_eq!(item["examFrequency"], 1);
        assert_eq!(item["displayExamFrequency"], 12);
        assert_eq!(item["examFamilyRoot"], "observe");
        assert_eq!(item["examFamilyMembers"]["observation"], 3);
    }

    #[test]
    fn kaoyan_derivational_family_index_covers_supplemental_members() {
        let source = serde_json::json!([{
            "headWord":"observe",
            "content":{"word":{"content":{"realExamFrequency":{
                "occurrences":6,
                "derivationalFamily":{
                    "root":"observe",
                    "occurrences":12,
                    "members":{
                        "observe":6,
                        "observation":3,
                        "observer":1,
                        "observable":1,
                        "observational":1
                    }
                }
            }}}}
        }])
        .to_string();

        let index = super::parse_exam_derivational_family_index(&source)
            .expect("parse derivational family index");

        assert_eq!(index.len(), 5);
        assert_eq!(index["observer"]["examFrequency"], 1);
        assert_eq!(index["observer"]["examFamilyFrequency"], 12);
        assert_eq!(index["observational"]["examFamilyRoot"], "observe");

        let portable = serde_json::json!({
            "observer":{
                "root":"observe",
                "occurrences":12,
                "strictOccurrences":1,
                "members":{"observe":6,"observation":3,"observer":1,"observable":1,"observational":1}
            }
        })
        .to_string();
        let portable_index = super::parse_exam_derivational_family_index(&portable)
            .expect("parse portable family index");
        assert_eq!(portable_index["observer"]["examFrequency"], 1);
        assert_eq!(portable_index["observer"]["examFamilyFrequency"], 12);
    }

    #[test]
    fn cloze_review_removes_inline_glosses_from_pure_purple_words() {
        let evidence = vec![serde_json::json!({
            "word":"observe", "mark":"unknown", "markScope":"prior"
        })];
        let mut result = serde_json::json!({
            "questions": [{
                "questionId": "q1",
                "annotatedContext": "They observe（观察） the change.",
                "candidates": []
            }]
        });

        super::remove_familiar_inline_glosses(&mut result, &evidence);

        assert_eq!(
            result["questions"][0]["annotatedContext"],
            "They observe the change."
        );
    }

    #[test]
    fn pure_purple_words_are_loaded_from_verified_prior_article_marks() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let prior_article =
            word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_article(
                &conn,
                &word_storage_core::models::ExerciseArticleDraft {
                    article_id: "paper-2007:cloze".to_string(),
                    source_type: "builtin".to_string(),
                    title: "Prior cloze".to_string(),
                    body: "They observe the change.".to_string(),
                    language: "en".to_string(),
                    metadata_json: "{}".to_string(),
                },
            )
            .expect("prior article");
        word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_vocab_occurrence(
            &conn,
            &word_storage_core::models::ExerciseVocabOccurrenceDraft {
                article_id: prior_article,
                entry_id: None,
                word_form: "observe".to_string(),
                normalized_form: "observe".to_string(),
                sentence_text: "They observe the change.".to_string(),
                paragraph_index: 0,
                sentence_index: 0,
                start_offset: 5,
                end_offset: 12,
                lookup_status: "matched".to_string(),
                user_mark: "unknown".to_string(),
                meaning_note: "观察".to_string(),
            },
        )
        .expect("prior occurrence");
        let mut evidence = Vec::new();

        super::augment_pure_purple_exam_evidence(
            &conn,
            "paper-2008:cloze",
            &std::collections::HashSet::from(["observe".to_string()]),
            &mut evidence,
        )
        .expect("augment evidence");

        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0]["markScope"], "prior");
        assert_eq!(evidence[0]["meaning"], "观察");
    }

    #[test]
    fn exam_causal_candidates_are_limited_to_marked_words_and_use_chinese_reasoning() {
        let evidence = vec![serde_json::json!({
            "word": "consequential",
            "mark": "unknown"
        })];
        let mut result = serde_json::json!({
            "candidates": [
                {
                    "word": "consequential",
                    "confidence": 0.8,
                    "reasoning": "It changes the option meaning."
                },
                {
                    "word": "ordinary",
                    "confidence": 0.9,
                    "reasoning": "Not marked by the user."
                }
            ]
        });

        super::retain_marked_exam_causal_candidates(&mut result, &evidence);

        let candidates = result["candidates"].as_array().expect("candidates");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0]["word"], "consequential");
        assert!(candidates[0]["reasoning"]
            .as_str()
            .is_some_and(|value| value.contains("标记词")));
    }

    #[test]
    fn causal_words_are_persisted_as_wrong_answer_priority_evidence() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let article_id =
            word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_article(
                &conn,
                &word_storage_core::models::ExerciseArticleDraft {
                    article_id: "paper-1:reading".to_string(),
                    source_type: "builtin".to_string(),
                    title: "Reading".to_string(),
                    body: "A consequential choice.".to_string(),
                    language: "en".to_string(),
                    metadata_json: "{}".to_string(),
                },
            )
            .expect("article");
        word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_vocab_occurrence(
            &conn,
            &word_storage_core::models::ExerciseVocabOccurrenceDraft {
                article_id,
                entry_id: None,
                word_form: "consequential".to_string(),
                normalized_form: "consequential".to_string(),
                sentence_text: "A consequential choice.".to_string(),
                paragraph_index: 0,
                sentence_index: 0,
                start_offset: 2,
                end_offset: 15,
                lookup_status: "matched".to_string(),
                user_mark: "unknown".to_string(),
                meaning_note: "重要的".to_string(),
            },
        )
        .expect("occurrence");
        conn.execute(
            "UPDATE exercise_vocab_occurrences
             SET user_mark = 'ignored', mark_level = 'familiar'
             WHERE article_id = ?1",
            [article_id],
        )
        .expect("set familiar mark");
        let evidence = super::load_exam_causal_evidence(&conn, "paper-1:reading")
            .expect("load causal evidence");
        assert_eq!(evidence[0]["word"], "consequential");
        assert_eq!(evidence[0]["mark"], "familiar");

        let promoted = super::persist_exam_causal_words(
            &conn,
            "paper-1:reading",
            &serde_json::json!({"candidates": [{"word": "consequential"}]}),
        )
        .expect("promote causal word");

        assert_eq!(promoted, vec!["consequential"]);
        let mark = conn
            .query_row(
                "SELECT user_mark FROM exercise_vocab_occurrences WHERE article_id = ?1",
                [article_id],
                |row| row.get::<_, String>(0),
            )
            .expect("load mark");
        assert_eq!(mark, "wrong");
    }

    #[test]
    fn new_word_selection_prioritizes_ai_causal_vocabulary() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute_batch(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'causal-study-priority', 'ready');
             INSERT INTO wordbooks
                (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 2, 1);
             INSERT INTO entries
                (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'ordinary', 'ordinary', 'adj.', 100.0),
                    (2, 1, 'causal', 'consequential', 'adj.', 1.0);
             INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1), (3, 2, 2);",
        )
        .expect("seed wordbook");
        let article_id =
            word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_article(
                &conn,
                &word_storage_core::models::ExerciseArticleDraft {
                    article_id: "paper-1:reading".to_string(),
                    source_type: "builtin".to_string(),
                    title: "Reading".to_string(),
                    body: "A consequential choice.".to_string(),
                    language: "en".to_string(),
                    metadata_json: "{}".to_string(),
                },
            )
            .expect("article");
        word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_vocab_occurrence(
            &conn,
            &word_storage_core::models::ExerciseVocabOccurrenceDraft {
                article_id,
                entry_id: Some(2),
                word_form: "consequential".to_string(),
                normalized_form: "consequential".to_string(),
                sentence_text: "A consequential choice.".to_string(),
                paragraph_index: 0,
                sentence_index: 0,
                start_offset: 2,
                end_offset: 15,
                lookup_status: "matched".to_string(),
                user_mark: "wrong".to_string(),
                meaning_note: "重要的".to_string(),
            },
        )
        .expect("causal occurrence");

        let selected = super::load_unlearned_ranked_entry_ids_for_wordbooks_on_date(
            &conn,
            &[3],
            2,
            "2026-07-27",
        )
        .expect("select new words");

        assert_eq!(selected.first(), Some(&2));
    }

    #[test]
    fn new_word_selection_prioritizes_reader_mark_levels() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        conn.execute_batch(
            "INSERT INTO source_versions (id, source_commit, status)
             VALUES (1, 'reader-mark-study-priority', 'ready');
             INSERT INTO wordbooks
                (id, code, name, source_version_id, total_entries, is_active)
             VALUES (3, 'KaoYan', 'KaoYan', 1, 2, 1);
             INSERT INTO entries
                (id, source_version_id, source_entry_key, word, part_of_speech, frequency)
             VALUES (1, 1, 'high_frequency', 'ordinary', 'adj.', 1000.0),
                    (2, 1, 'reader_marked', 'consequential', 'adj.', 1.0);
             INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1), (3, 2, 2);",
        )
        .expect("seed wordbook");
        let article_id =
            word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_article(
                &conn,
                &word_storage_core::models::ExerciseArticleDraft {
                    article_id: "paper-2:reading".to_string(),
                    source_type: "builtin".to_string(),
                    title: "Reading".to_string(),
                    body: "A consequential choice.".to_string(),
                    language: "en".to_string(),
                    metadata_json: "{}".to_string(),
                },
            )
            .expect("article");
        word_storage_core::persistence::exercise_vocab_repo::upsert_exercise_vocab_occurrence(
            &conn,
            &word_storage_core::models::ExerciseVocabOccurrenceDraft {
                article_id,
                entry_id: Some(2),
                word_form: "consequential".to_string(),
                normalized_form: "consequential".to_string(),
                sentence_text: "A consequential choice.".to_string(),
                paragraph_index: 0,
                sentence_index: 0,
                start_offset: 2,
                end_offset: 15,
                lookup_status: "matched".to_string(),
                user_mark: "ignored".to_string(),
                meaning_note: "重要的".to_string(),
            },
        )
        .expect("reader mark occurrence");
        conn.execute(
            "UPDATE exercise_vocab_occurrences
             SET mark_level = 'unknown'
             WHERE article_id = ?1",
            [article_id],
        )
        .expect("set unknown mark level");

        let selected = super::load_unlearned_ranked_entry_ids_for_wordbooks_on_date(
            &conn,
            &[3],
            2,
            "2026-07-28",
        )
        .expect("select new words");

        assert_eq!(selected.first(), Some(&2));
    }

    #[test]
    fn exam_analysis_tasks_persist_completion_and_clear_unread_state() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let running = serde_json::json!({
            "taskId": "task-1",
            "paperTitle": "Kaoyan English I 2010",
            "sectionTitle": "Reading Text 1",
            "status": "running",
            "createdAt": "2026-07-27T10:00:00Z",
            "findings": []
        });
        super::upsert_exam_analysis_task_with_connection(&conn, running)
            .expect("save running task");
        let completed = serde_json::json!({
            "taskId": "task-1",
            "paperTitle": "Kaoyan English I 2010",
            "sectionTitle": "Reading Text 1",
            "status": "completed",
            "createdAt": "2026-07-27T10:00:00Z",
            "completedAt": "2026-07-27T10:01:00Z",
            "findings": [{"word": "consequential"}]
        });
        super::upsert_exam_analysis_task_with_connection(&conn, completed).expect("complete task");

        let inbox = super::list_exam_analysis_tasks_with_connection(&conn).expect("list tasks");
        assert_eq!(inbox["unreadCount"], 1);
        assert_eq!(inbox["items"][0]["status"], "completed");

        super::mark_exam_analysis_tasks_read_with_connection(&conn).expect("mark read");
        let read = super::list_exam_analysis_tasks_with_connection(&conn).expect("list read tasks");
        assert_eq!(read["unreadCount"], 0);
        assert_eq!(read["items"][0]["unread"], false);
    }

    #[test]
    fn exam_analysis_tasks_keep_only_latest_retry_for_same_section() {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        for (task_id, created_at) in [
            ("task-old", "2026-08-17T10:00:00Z"),
            ("task-new", "2026-08-17T10:05:00Z"),
        ] {
            super::upsert_exam_analysis_task_with_connection(
                &conn,
                serde_json::json!({
                    "taskId": task_id,
                    "exam": "kaoyan-english-1",
                    "paperId": "kaoyan-english-1-2008",
                    "sectionId": "reading-4",
                    "status": "failed",
                    "createdAt": created_at
                }),
            )
            .expect("save retry");
        }

        let inbox = super::list_exam_analysis_tasks_with_connection(&conn).expect("list tasks");
        assert_eq!(inbox["items"].as_array().unwrap().len(), 1);
        assert_eq!(inbox["items"][0]["taskId"], "task-new");
    }

    #[test]
    fn structured_review_uses_local_question_and_option_facts() {
        let mut result = serde_json::json!({
            "questions": [{
                "questionId": "q1",
                "questionNumber": 999,
                "stem": "invented",
                "selectedAnswer": "Z",
                "correctAnswer": "Y",
                "optionAnalysis": [
                    {"label": "A", "meaning": "invented", "analysis": "干扰项分析"}
                ],
                "analysis": "结合文章与答案的分析",
                "candidates": []
            }]
        });
        let questions = vec![super::PreparedExamSectionQuestion {
            question_id: "q1".to_string(),
            question_number: 21,
            stem: "What changed?".to_string(),
            choices: vec![
                serde_json::json!({"label": "A", "text": "local option A"}),
                serde_json::json!({"label": "B", "text": "local option B"}),
            ],
            correct_answer: "B".to_string(),
            attempt_id: "attempt-1".to_string(),
            selected_answer: "A".to_string(),
        }];

        super::attach_local_structured_review_facts(&mut result, &questions);

        let question = &result["questions"][0];
        assert_eq!(question["questionNumber"], 21);
        assert_eq!(question["stem"], "What changed?");
        assert_eq!(question["selectedAnswer"], "A");
        assert_eq!(question["correctAnswer"], "B");
        assert_eq!(question["optionAnalysis"][0]["meaning"], "local option A");
        assert_eq!(question["optionAnalysis"][0]["analysis"], "干扰项分析");
        assert_eq!(question["optionAnalysis"][1]["meaning"], "local option B");
    }

    #[test]
    fn exam_word_lookup_resolves_common_inflected_forms() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        conn.execute_batch(
            "CREATE TABLE entries (
                id INTEGER PRIMARY KEY,
                word TEXT NOT NULL,
                lemma TEXT NOT NULL DEFAULT '',
                frequency INTEGER NOT NULL DEFAULT 0
             );
             INSERT INTO entries (id, word, lemma, frequency)
             VALUES (1, 'habit', 'habit', 10), (2, 'researcher', 'researcher', 8),
                    (3, 'paradox', 'paradox', 7), (4, 'patent', 'patent', 6),
                    (5, 'authorize', 'authorize', 5);
             CREATE TABLE entry_aliases (
                entry_id INTEGER NOT NULL,
                alias TEXT NOT NULL,
                meaning_cn TEXT NOT NULL,
                source_kind TEXT NOT NULL
             );",
        )
        .expect("seed entries");
        let word_content = serde_json::json!({
            "relWord": {"rels": [{"words": [{
                "hwd": "paradoxical",
                "tran": "矛盾的；似非而是的"
            }]}]},
            "syno": {"synos": [{
                "tran": "矛盾的；似非而是的",
                "hwds": [{"w": "contradictory"}]
            }]}
        });
        super::persist_seed_entry_aliases(&conn, 3, Some(&word_content))
            .expect("persist related alias");

        assert_eq!(
            super::find_entry_id_by_word(&conn, "habits").unwrap(),
            Some(1)
        );
        assert_eq!(
            super::find_entry_id_by_word(&conn, "researchers").unwrap(),
            Some(2)
        );
        assert_eq!(
            super::find_entry_id_by_word(&conn, "paradoxical").unwrap(),
            Some(3)
        );
        assert_eq!(
            super::resolve_exam_word_family(&conn, "patents").unwrap(),
            (Some(4), "patent".to_string())
        );
        assert_eq!(
            super::resolve_exam_word_family(&conn, "authorized").unwrap(),
            (Some(5), "authorize".to_string())
        );
        assert_eq!(
            super::load_entry_alias_meaning_strings(&conn, "paradoxical").unwrap(),
            vec![serde_json::Value::String("矛盾的；似非而是的".to_string())]
        );
        assert_eq!(
            conn.query_row(
                "SELECT source_kind FROM entry_aliases WHERE entry_id = 3 AND alias = 'contradictory'",
                [],
                |row| row.get::<_, String>(0),
            )
            .expect("persisted explicit synonym"),
            "synonym"
        );
    }

    #[test]
    fn study_derivational_family_uses_related_word_alias_meanings() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        conn.execute_batch(
            "CREATE TABLE entries (
                id INTEGER PRIMARY KEY,
                word TEXT NOT NULL,
                lemma TEXT NOT NULL DEFAULT '',
                frequency INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE entry_aliases (
                entry_id INTEGER NOT NULL,
                alias TEXT NOT NULL,
                meaning_cn TEXT NOT NULL,
                source_kind TEXT NOT NULL
             );
             INSERT INTO entries (id, word, lemma, frequency)
             VALUES (1, 'observe', 'observe', 20);
             INSERT INTO entry_aliases (entry_id, alias, meaning_cn, source_kind)
             VALUES
                (1, 'observation', '观察；观察结果', 'related_word'),
                (1, 'observer', '观察者', 'related_word'),
                (1, 'observer', '观察者', 'related_word'),
                (1, 'observed', '观察', 'inflection');",
        )
        .expect("seed family");
        let mut question = serde_json::json!({
            "entrySourceId": "1",
            "word": "observe"
        });

        super::enrich_study_question_derivational_family(&conn, &mut question)
            .expect("enrich family");

        assert_eq!(
            question["derivationalFamily"],
            serde_json::json!([
                {"word": "observation", "meaning": "观察；观察结果"},
                {"word": "observer", "meaning": "观察者"}
            ])
        );
    }

    #[test]
    fn study_derivational_family_separates_real_and_realize_branches() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        conn.execute_batch(
            "CREATE TABLE entries (
                id INTEGER PRIMARY KEY,
                word TEXT NOT NULL,
                lemma TEXT NOT NULL DEFAULT '',
                frequency INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE entry_aliases (
                entry_id INTEGER NOT NULL,
                alias TEXT NOT NULL,
                meaning_cn TEXT NOT NULL,
                source_kind TEXT NOT NULL
             );
             INSERT INTO entries (id, word, lemma, frequency)
             VALUES (1, 'reality', 'reality', 20);
             INSERT INTO entry_aliases (entry_id, alias, meaning_cn, source_kind)
             VALUES
                (1, 'real', '真实的', 'related_word'),
                (1, 'real', '现实', 'related_word'),
                (1, 'really', '真正地', 'related_word'),
                (1, 'realize', '实现', 'related_word'),
                (1, 'realization', '实现；领悟', 'related_word');",
        )
        .expect("seed family");
        let mut question = serde_json::json!({
            "entrySourceId": "1",
            "word": "reality"
        });

        super::enrich_study_question_derivational_family(&conn, &mut question)
            .expect("enrich family");

        assert_eq!(
            question["derivationalFamily"],
            serde_json::json!([
                {"word": "real", "meaning": "现实；真实的"},
                {"word": "really", "meaning": "真正地"}
            ])
        );
    }

    #[test]
    fn study_synonyms_group_shared_glosses_and_require_matching_part_of_speech() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        conn.execute_batch(
            "INSERT INTO source_versions (id, source_commit, status) VALUES (1, 'synonym-test', 'ready');
             INSERT INTO wordbooks (id, code, name, category, source_version_id)
             VALUES (3, 'kaoyan', 'KaoYan', 'exam', 1);
             INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma, exam_frequency, exam_rank)
             VALUES
                (1, 1, 'assist', 'assist', 'assist', 10, 1),
                (2, 1, 'aid', 'aid', 'aid', 9, 2),
                (3, 1, 'help', 'help', 'help', 8, 3),
                (4, 1, 'support', 'support', 'support', 0, 4),
                (5, 1, 'assistive', 'assistive', 'assistive', 7, 5);
             INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1), (3, 2, 2), (3, 3, 3), (3, 4, 4), (3, 5, 5);
             INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES
                (1, 'vt', '帮助；协助', 0),
                (2, 'vt', '帮助；援助', 0),
                (3, 'vt', '协助；帮忙', 0),
                (4, 'vt', '帮助', 0),
                (5, 'adj', '辅助的', 0);",
        )
        .expect("seed high-frequency synonyms");
        let mut question = serde_json::json!({
            "entrySourceId": "assist",
            "word": "assist"
        });

        super::enrich_study_question_synonym_groups(&conn, &mut question).expect("enrich synonyms");

        assert_eq!(
            question["synonymGroups"],
            serde_json::json!([
                {"meaning": "帮助", "words": ["aid"]},
                {"meaning": "协助", "words": ["help"]}
            ])
        );
    }

    #[test]
    fn study_synonyms_include_explicit_seed_dictionary_relations() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("schema");
        conn.execute_batch(
            "INSERT INTO source_versions (id, source_commit, status) VALUES (1, 'explicit-synonym-test', 'ready');
             INSERT INTO wordbooks (id, code, name, category, source_version_id)
             VALUES (3, 'kaoyan', 'KaoYan', 'exam', 1);
             INSERT INTO entries (id, source_version_id, source_entry_key, word, lemma, exam_frequency, exam_rank)
             VALUES
                (1, 1, 'attract', 'attract', 'attract', 10, 1),
                (2, 1, 'engage', 'engage', 'engage', 9, 2),
                (3, 1, 'absorb', 'absorb', 'absorb', 8, 3),
                (4, 1, 'cause', 'cause', 'cause', 7, 4),
                (5, 1, 'produce', 'produce', 'produce', 0, 5);
             INSERT INTO wordbook_entries (wordbook_id, entry_id, rank_in_book)
             VALUES (3, 1, 1), (3, 2, 2), (3, 3, 3), (3, 4, 4), (3, 5, 5);
             INSERT INTO entry_meanings (entry_id, pos, meaning_cn, sort_order)
             VALUES
                (1, 'vt', '吸引；引起', 0),
                (2, 'vt', '参与；吸引', 0),
                (3, 'vt', '吸收；吸引', 0),
                (4, 'vt', '引起；导致', 0),
                (5, 'vt', '引起；生产', 0);
             INSERT INTO entry_aliases (entry_id, alias, meaning_cn, source_kind)
             VALUES
                (1, 'engage', '吸引；引起', 'synonym'),
                (1, 'absorb', '吸引；引起', 'synonym'),
                (1, 'produce', '吸引；引起', 'synonym');",
        )
        .expect("seed explicit synonyms");
        let mut question = serde_json::json!({
            "entrySourceId": "attract",
            "word": "attract"
        });

        super::enrich_study_question_synonym_groups(&conn, &mut question).expect("enrich synonyms");

        assert_eq!(
            question["synonymGroups"],
            serde_json::json!([
                {"meaning": "吸引", "words": ["engage", "absorb"]},
                {"meaning": "引起", "words": ["cause"]}
            ])
        );
    }

    #[test]
    fn exam_phrase_entries_are_idempotent_and_queryable() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");

        super::ensure_exam_dictionary_entries(&conn, None).expect("seed phrases");
        super::ensure_exam_dictionary_entries(&conn, None).expect("seed phrases again");

        let entry_id = super::find_entry_id_by_word(&conn, "so long as")
            .expect("lookup phrase")
            .expect("phrase entry");
        assert_eq!(
            super::load_entry_meaning_strings(&conn, entry_id).unwrap(),
            vec![serde_json::Value::String("只要".to_string())]
        );
        let upon_id = super::find_entry_id_by_word(&conn, "upon")
            .expect("lookup supplemental word")
            .expect("supplemental word entry");
        assert!(super::load_entry_meaning_strings(&conn, upon_id)
            .unwrap()
            .iter()
            .any(|meaning| meaning
                .as_str()
                .is_some_and(|value| value.contains("在……之上"))));
        assert_eq!(
            super::resolve_exam_mark_meaning(&conn, "upon", "", None).unwrap(),
            "在……之上；在……之后"
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM entries WHERE word = 'so long as'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn exam_corpus_dictionary_imports_basic_words_and_inflected_phrases_once() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let bundle_dir = env::temp_dir().join(format!("exam-corpus-dictionary-{nonce}"));
        let dictionary_dir = bundle_dir.join("exam-dictionary");
        fs::create_dir_all(&dictionary_dir).expect("create dictionary fixture");
        fs::write(
            dictionary_dir.join("exam-corpus-dictionary.json"),
            serde_json::json!({
                "schemaVersion": 1,
                "source": {"commit": "fixture"},
                "entries": [
                    {
                        "term": "the",
                        "lemma": "the",
                        "partOfSpeech": "article",
                        "meaning": "这；该"
                    },
                    {
                        "term": "looked at",
                        "lemma": "look at",
                        "partOfSpeech": "phrase",
                        "meaning": "查看；考虑"
                    }
                ]
            })
            .to_string(),
        )
        .expect("write dictionary fixture");

        super::ensure_exam_dictionary_entries(&conn, Some(&bundle_dir))
            .expect("seed curated dictionary");
        super::resolve_exam_dictionary_term(&conn, "the", Some(&bundle_dir))
            .expect("resolve basic corpus word");
        super::resolve_exam_dictionary_term(&conn, "looked at", Some(&bundle_dir))
            .expect("resolve inflected corpus phrase");
        super::resolve_exam_dictionary_term(&conn, "looked at", Some(&bundle_dir))
            .expect("resolve imported corpus phrase again");

        let the_id = super::find_entry_id_by_word(&conn, "the")
            .expect("lookup basic word")
            .expect("basic word entry");
        assert_eq!(
            super::load_entry_meaning_strings(&conn, the_id).unwrap(),
            vec![serde_json::Value::String("这；该".to_string())]
        );
        let phrase_id = super::find_entry_id_by_word(&conn, "look at")
            .expect("lookup phrase lemma")
            .expect("phrase entry");
        assert_eq!(
            super::load_entry_meaning_strings(&conn, phrase_id).unwrap(),
            vec![serde_json::Value::String("查看；考虑".to_string())]
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM entries WHERE source_entry_key LIKE 'exam_corpus:%'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            2
        );

        fs::remove_dir_all(bundle_dir).expect("cleanup dictionary fixture");
    }

    #[test]
    fn bundled_exam_corpus_dictionary_loads_quickly_and_imports_only_lookups() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        let bundle_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/flutter_mobile/assets");
        let started = std::time::Instant::now();

        let (_, the_meanings) =
            super::resolve_exam_dictionary_term(&conn, "the", Some(&bundle_dir))
                .expect("resolve bundled basic word");
        let (_, phrase_meanings) =
            super::resolve_exam_dictionary_term(&conn, "looked at", Some(&bundle_dir))
                .expect("resolve bundled phrase");

        let imported: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM entries WHERE source_entry_key LIKE 'exam_corpus:%'",
                [],
                |row| row.get(0),
            )
            .expect("count bundled dictionary entries");
        assert_eq!(imported, 2);
        assert!(the_meanings.iter().any(|value| value.as_str().is_some()));
        assert!(phrase_meanings.iter().any(|value| value.as_str().is_some()));
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        eprintln!(
            "bundled exam dictionary lazy load completed in {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn unlisted_phrase_falls_back_to_clearly_labeled_word_meanings() {
        let conn = rusqlite::Connection::open_in_memory().expect("open db");
        word_storage_core::persistence::schema::apply_schema(&conn).expect("apply schema");
        seed_basic_entry(&conn, "social", "社会的");
        seed_basic_entry(&conn, "media", "媒体");

        assert_eq!(
            super::build_exam_phrase_fallback_meaning(&conn, "social media", None).unwrap(),
            Some("未收录固定词组；逐词：social（社会的） / media（媒体）".to_string())
        );
        assert_eq!(
            super::build_exam_phrase_fallback_meaning(&conn, "unknown phrase", None).unwrap(),
            None
        );
    }

    #[test]
    fn exam_report_classifies_supported_objective_sections() {
        assert_eq!(
            super::exam_objective_section_kind("listening", "Listening"),
            Some("listening")
        );
        assert_eq!(
            super::exam_objective_section_kind("section", "完型填空"),
            Some("cloze")
        );
        assert_eq!(
            super::exam_objective_section_kind("part-b", "阅读理解 B 新题型"),
            Some("newType")
        );
        assert_eq!(
            super::exam_objective_section_kind("text1", "阅读 Text 1"),
            Some("reading")
        );
        assert_eq!(
            super::exam_objective_section_kind("part-a", "写作 Part A"),
            None
        );
    }
}
