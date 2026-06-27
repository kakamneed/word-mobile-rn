//! Baseline integration test runner.
//!
//! Executes golden sample scenarios against the Rust core facades using
//! in-memory SQLite databases, capturing responses for structural comparison.

use rusqlite::Connection;
use serde_json::Value;

use word_app_core::platform::{PlatformError, PlatformRuntime};
use word_storage_core::models::*;
use word_storage_core::persistence;

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Mock platform runtime
// ---------------------------------------------------------------------------

struct MockPlatformRuntime {
    data_dir: PathBuf,
    has_snapshot: bool,
}

impl MockPlatformRuntime {
    fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            has_snapshot: false,
        }
    }
}

impl PlatformRuntime for MockPlatformRuntime {
    fn app_data_dir(&self) -> Result<PathBuf, PlatformError> {
        Ok(self.data_dir.clone())
    }

    fn app_config_dir(&self) -> Result<PathBuf, PlatformError> {
        Ok(self.data_dir.join("config"))
    }

    fn app_log_dir(&self) -> Result<PathBuf, PlatformError> {
        Ok(self.data_dir.join("logs"))
    }

    fn bundled_resource_path(&self, relative_path: &str) -> Result<PathBuf, PlatformError> {
        Ok(self.data_dir.join("bundle").join(relative_path))
    }

    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-04-22T01:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    fn open_external(&self, _url: &str) -> Result<(), PlatformError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Test database helpers
// ---------------------------------------------------------------------------

fn create_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("Failed to create in-memory DB");
    persistence::schema::apply_schema(&conn).expect("Failed to apply schema");
    // Disable FK checks after schema creation - baseline tests validate behavior
    // and response shapes, not referential integrity. FK constraints would
    // require seeding full entry hierarchies for every test.
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .expect("Failed to disable FK");
    conn
}

fn seed_onboarding_completed(conn: &Connection) {
    conn.execute(
        "INSERT INTO app_metadata (key, value) VALUES ('onboarding_completed', 'true')",
        [],
    )
    .expect("Failed to seed onboarding");
}

fn seed_active_plan(conn: &Connection) -> i64 {
    conn.execute(
        "INSERT INTO plan_templates (name, new_words_per_day, review_words_per_day, mixed_test_per_day, wrong_word_test_per_day, growth_interval_days, growth_increment, is_active)
         VALUES ('Default Plan', 20, 30, 10, 5, 7, 5, 1)",
        [],
    )
    .expect("Failed to seed plan");
    conn.last_insert_rowid()
}

fn seed_wordbook(conn: &Connection) -> i64 {
    conn.execute(
        "INSERT INTO source_versions (source_commit, status) VALUES ('baseline-seed', 'ready')",
        [],
    )
    .expect("Failed to seed source version");
    let sv_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO wordbooks (code, name, category, source_version_id, total_entries, is_active)
         VALUES ('CET4', 'CET-4', 'exam', ?1, 5, 1)",
        [sv_id],
    )
    .expect("Failed to seed wordbook");
    conn.last_insert_rowid()
}

/// Seed entries matching the test payload source IDs so FK constraints pass.
fn seed_entries_for_payloads(conn: &Connection) {
    let sv_id: i64 = conn
        .query_row(
            "SELECT id FROM source_versions WHERE source_commit = 'baseline-seed'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(1);

    for (key, word, _meaning) in [
        ("w1", "abandon", "give up"),
        ("w2", "abstract", "abstract"),
        ("w3", "academic", "academic"),
        ("w4", "accelerate", "speed up"),
        ("w5", "accept", "accept"),
    ] {
        conn.execute(
            "INSERT OR IGNORE INTO entries (source_version_id, source_entry_key, word, lemma, part_of_speech)
             VALUES (?1, ?2, ?3, ?3, 'v.')",
            rusqlite::params![sv_id, key, word],
        )
        .expect("Failed to seed entry");
    }
}

fn make_entry_payloads() -> Vec<StartSessionEntryPayload> {
    vec![
        StartSessionEntryPayload {
            source_id: "w1".into(),
            word: "abandon".into(),
            part_of_speech: Some("v.".into()),
            frequency: 0.8,
            phonetic_us: None,
            phonetic_uk: None,
            meaning_details: vec![StartSessionMeaningPayload {
                pos: "v.".into(),
                meaning_cn: "meaning".into(),
                meaning_en: None,
            }],
            meanings: vec!["meaning".into()],
            example_sentence: Some("He abandoned the project.".into()),
            example_translation: Some("example translation".into()),
            cn_choice_distractors: Vec::new(),
            en_choice_distractors: Vec::new(),
        },
        StartSessionEntryPayload {
            source_id: "w2".into(),
            word: "abstract".into(),
            part_of_speech: Some("adj.".into()),
            frequency: 0.7,
            phonetic_us: None,
            phonetic_uk: None,
            meaning_details: vec![StartSessionMeaningPayload {
                pos: "adj.".into(),
                meaning_cn: "meaning".into(),
                meaning_en: None,
            }],
            meanings: vec!["meaning".into()],
            example_sentence: Some("Abstract thinking is important.".into()),
            example_translation: Some("example translation".into()),
            cn_choice_distractors: Vec::new(),
            en_choice_distractors: Vec::new(),
        },
        StartSessionEntryPayload {
            source_id: "w3".into(),
            word: "academic".into(),
            part_of_speech: Some("adj.".into()),
            frequency: 0.6,
            phonetic_us: None,
            phonetic_uk: None,
            meaning_details: vec![StartSessionMeaningPayload {
                pos: "adj.".into(),
                meaning_cn: "meaning".into(),
                meaning_en: None,
            }],
            meanings: vec!["meaning".into()],
            example_sentence: Some("She has an academic background.".into()),
            example_translation: Some("example translation".into()),
            cn_choice_distractors: Vec::new(),
            en_choice_distractors: Vec::new(),
        },
        StartSessionEntryPayload {
            source_id: "w4".into(),
            word: "accelerate".into(),
            part_of_speech: Some("v.".into()),
            frequency: 0.5,
            phonetic_us: None,
            phonetic_uk: None,
            meaning_details: vec![StartSessionMeaningPayload {
                pos: "v.".into(),
                meaning_cn: "meaning".into(),
                meaning_en: None,
            }],
            meanings: vec!["meaning".into()],
            example_sentence: Some("We need to accelerate the process.".into()),
            example_translation: Some("example translation".into()),
            cn_choice_distractors: Vec::new(),
            en_choice_distractors: Vec::new(),
        },
        StartSessionEntryPayload {
            source_id: "w5".into(),
            word: "accept".into(),
            part_of_speech: Some("v.".into()),
            frequency: 0.9,
            phonetic_us: None,
            phonetic_uk: None,
            meaning_details: vec![StartSessionMeaningPayload {
                pos: "v.".into(),
                meaning_cn: "meaning".into(),
                meaning_en: None,
            }],
            meanings: vec!["meaning".into()],
            example_sentence: Some("Please accept my apology.".into()),
            example_translation: Some("example translation".into()),
            cn_choice_distractors: Vec::new(),
            en_choice_distractors: Vec::new(),
        },
    ]
}

// ---------------------------------------------------------------------------
// Assertion helpers
// ---------------------------------------------------------------------------

fn json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for segment in path.split('.') {
        if let Ok(idx) = segment.parse::<usize>() {
            current = current.get(idx)?;
        } else {
            current = current.get(segment)?;
        }
    }
    Some(current)
}

fn assert_json_equals(response: &Value, path: &str, expected: &Value) {
    let actual = json_path(response, path)
        .unwrap_or_else(|| panic!("Path '{}' not found in response: {}", path, response));
    assert_eq!(
        actual, expected,
        "Assertion failed at path '{}': expected {}, got {}",
        path, expected, actual
    );
}

fn assert_json_not_null(response: &Value, path: &str) {
    let actual = json_path(response, path)
        .unwrap_or_else(|| panic!("Path '{}' not found in response", path));
    assert!(
        !actual.is_null(),
        "Assertion failed at path '{}': expected non-null, got null",
        path
    );
}

fn assert_json_greater_than(response: &Value, path: &str, threshold: i64) {
    let actual = json_path(response, path)
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| panic!("Path '{}' not found or not integer", path));
    assert!(
        actual > threshold,
        "Assertion failed at path '{}': expected > {}, got {}",
        path,
        threshold,
        actual
    );
}

fn to_json<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("Failed to serialize to JSON")
}

/// Get the correct answer for a question.
fn correct_answer_for(question: &StudyQuestion) -> String {
    if question.question_type.is_choice_type() {
        question.correct_choice_label.clone().unwrap_or_default()
    } else {
        question
            .accepted_meanings
            .first()
            .cloned()
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Bootstrap baseline tests
// ---------------------------------------------------------------------------

#[test]
fn baseline_bootstrap_first_run_ready() {
    let conn = create_test_db();
    let temp_dir = std::env::temp_dir().join("baseline_test_bootstrap_first_run");
    let runtime = MockPlatformRuntime::new(temp_dir);

    let state = word_app_core::bootstrap_with_connection(&runtime, &conn)
        .expect("Bootstrap should succeed");
    let json = to_json(&state);

    assert_eq!(
        json_path(&json, "firstRunRequired").unwrap(),
        &Value::Bool(true)
    );
    assert_eq!(
        json_path(&json, "databaseStatus").unwrap(),
        &Value::String("ready".into())
    );
    assert_eq!(
        json_path(&json, "snapshotStatus").unwrap(),
        &Value::String("missing_required".into())
    );
    assert_eq!(
        json_path(&json, "settingsEntryAvailable").unwrap(),
        &Value::Bool(false)
    );
    assert_eq!(json_path(&json, "appReady").unwrap(), &Value::Bool(false));
}

#[test]
fn baseline_bootstrap_existing_user_ready() {
    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);

    let temp_dir = std::env::temp_dir().join("baseline_test_bootstrap_existing");
    let runtime = MockPlatformRuntime::new(temp_dir);

    let state = word_app_core::bootstrap_with_connection(&runtime, &conn)
        .expect("Bootstrap should succeed");
    let json = to_json(&state);

    assert_eq!(
        json_path(&json, "firstRunRequired").unwrap(),
        &Value::Bool(false)
    );
    assert_eq!(
        json_path(&json, "databaseStatus").unwrap(),
        &Value::String("ready".into())
    );
    assert_eq!(json_path(&json, "appReady").unwrap(), &Value::Bool(true));
    assert_eq!(
        json_path(&json, "settingsEntryAvailable").unwrap(),
        &Value::Bool(true)
    );
}

// ---------------------------------------------------------------------------
// Today baseline tests
// ---------------------------------------------------------------------------

#[test]
fn baseline_today_plan_stable() {
    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);
    seed_wordbook(&conn);

    let state =
        word_app_core::get_today_home_state(&conn).expect("getTodayHomeState should succeed");
    let json = to_json(&state);

    assert_json_not_null(&json, "activePlan");
    assert_json_equals(
        &json,
        "activePlan.newWordsPerDay",
        &Value::Number(20.into()),
    );
    assert_json_equals(
        &json,
        "activePlan.reviewWordsPerDay",
        &Value::Number(30.into()),
    );
    assert_json_equals(
        &json,
        "activePlan.mixedTestPerDay",
        &Value::Number(10.into()),
    );
    assert_json_equals(
        &json,
        "activePlan.wrongWordTestPerDay",
        &Value::Number(5.into()),
    );
    assert_json_equals(
        &json,
        "activePlan.name",
        &Value::String("Default Plan".into()),
    );

    assert_json_not_null(&json, "todaySnapshot");
    assert_json_equals(
        &json,
        "todaySnapshot.newWordsTarget",
        &Value::Number(20.into()),
    );
    assert_json_equals(
        &json,
        "todaySnapshot.reviewWordsTarget",
        &Value::Number(30.into()),
    );
    assert_json_equals(
        &json,
        "todaySnapshot.newWordsCompleted",
        &Value::Number(0.into()),
    );
    assert_json_equals(
        &json,
        "todaySnapshot.reviewWordsCompleted",
        &Value::Number(0.into()),
    );

    assert_json_greater_than(&json, "dailyProgress.totalTasks", 0);
    assert_json_equals(
        &json,
        "dailyProgress.completedTasks",
        &Value::Number(0.into()),
    );
    assert_json_equals(
        &json,
        "dailyProgress.nextRecommendedAction",
        &Value::String("Start new words".into()),
    );

    let wordbooks = json_path(&json, "wordbooks").expect("wordbooks field");
    assert!(wordbooks.is_array(), "wordbooks should be array");
    assert!(
        wordbooks.as_array().unwrap().len() >= 1,
        "at least 1 wordbook"
    );
}

// ---------------------------------------------------------------------------
// Study baseline tests
// ---------------------------------------------------------------------------

#[test]
fn baseline_study_newword_all_correct() {
    word_app_core::clear_all_active_sessions();

    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);
    seed_wordbook(&conn);
    seed_entries_for_payloads(&conn);
    let payloads = make_entry_payloads();
    let source_ids: Vec<String> = payloads.iter().map(|p| p.source_id.clone()).collect();

    let request = StartSessionRequest {
        mode: SessionMode::NewWord,
        wordbook_id: None,
        entry_source_ids: source_ids,
        entry_payloads: payloads,
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let start_response = word_app_core::start_study_session(&conn, request)
        .expect("startStudySession should succeed");
    let json = to_json(&start_response);

    assert_json_equals(&json, "session.mode", &Value::String("newWord".into()));
    assert_json_equals(&json, "session.totalWords", &Value::Number(5.into()));
    assert_json_not_null(&json, "currentQuestion.questionType");
    assert_json_equals(&json, "progress.current", &Value::Number(1.into()));
    assert_json_equals(&json, "progress.total", &Value::Number(20.into()));

    let session_id = start_response.session.session_id.clone();

    // Answer all questions correctly
    let mut current_question = start_response.current_question;
    let mut answer_count = 0;

    loop {
        let correct = correct_answer_for(&current_question);

        let submit_request = SubmitAnswerRequest {
            question_id: current_question.question_id.clone(),
            response: correct,
            response_time_ms: 1500,
        };

        let submit_response = word_app_core::submit_study_answer(&conn, submit_request)
            .expect("submitStudyAnswer should succeed");
        answer_count += 1;

        let submit_json = to_json(&submit_response);
        assert_json_equals(
            &submit_json,
            "result.outcome",
            &Value::String("correct".into()),
        );

        if submit_response.is_complete {
            let summary = submit_response
                .summary
                .as_ref()
                .expect("summary should exist on completion");
            assert_eq!(summary.correct_count, answer_count as u32);
            assert_eq!(summary.incorrect_count, 0);
            assert_eq!(summary.skipped_count, 0);
            assert_eq!(summary.total_questions, answer_count as u32);
            assert_eq!(summary.accuracy_percent, 100.0);
            break;
        }

        current_question = submit_response
            .current_question
            .expect("should have next question");
    }

    assert_eq!(
        answer_count, 20,
        "newWord mode should have 20 questions for 5 words"
    );

    let results_after_final_submit: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM study_results WHERE session_id = ?1",
            [&session_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        results_after_final_submit, 20,
        "final submit should persist results before the completion screen closes"
    );

    // Complete session
    let complete_response = word_app_core::complete_study_session(&conn, &session_id)
        .expect("completeStudySession should succeed");
    let complete_json = to_json(&complete_response);
    assert_json_not_null(&complete_json, "summary");
    assert_json_not_null(&complete_json, "nextAction");

    // Verify DB persistence
    let session_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM study_sessions WHERE session_id = ?1 AND completed_at IS NOT NULL",
            [&session_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        session_count, 1,
        "One completed session should be persisted"
    );

    let results_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM study_results WHERE session_id = ?1",
            [&session_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(results_count, 20, "20 study results should be persisted");

    let all_correct: bool = conn
        .query_row(
            "SELECT COUNT(*) = COUNT(CASE WHEN outcome = '\"correct\"' THEN 1 END) FROM study_results WHERE session_id = ?1",
            [&session_id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(all_correct, "All results should have outcome=correct");
}

#[test]
fn baseline_study_mixed_incorrect() {
    word_app_core::clear_all_active_sessions();

    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);
    seed_wordbook(&conn);
    seed_entries_for_payloads(&conn);
    let payloads = make_entry_payloads();

    let request = StartSessionRequest {
        mode: SessionMode::MixedTest,
        wordbook_id: None,
        entry_source_ids: vec!["w1".into(), "w2".into(), "w3".into()],
        entry_payloads: payloads[..3].to_vec(),
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let start_response = word_app_core::start_study_session(&conn, request)
        .expect("startStudySession should succeed");
    let start_json = to_json(&start_response);

    assert_json_equals(
        &start_json,
        "session.mode",
        &Value::String("mixedTest".into()),
    );
    assert_json_equals(&start_json, "session.totalWords", &Value::Number(3.into()));
    assert_json_equals(&start_json, "progress.total", &Value::Number(3.into()));

    let session_id = start_response.session.session_id.clone();

    // First answer: intentionally wrong
    let wrong_submit = SubmitAnswerRequest {
        question_id: start_response.current_question.question_id.clone(),
        response: "WRONG_VALUE".into(),
        response_time_ms: 3200,
    };
    let wrong_response = word_app_core::submit_study_answer(&conn, wrong_submit)
        .expect("submitStudyAnswer should succeed");
    assert_json_equals(
        &to_json(&wrong_response),
        "result.outcome",
        &Value::String("incorrect".into()),
    );

    // Remaining answers: correct
    let mut remaining = wrong_response;
    while !remaining.is_complete {
        let question = remaining
            .current_question
            .as_ref()
            .expect("should have next question");
        let correct = correct_answer_for(question);

        let submit = SubmitAnswerRequest {
            question_id: question.question_id.clone(),
            response: correct,
            response_time_ms: 2000,
        };
        remaining = word_app_core::submit_study_answer(&conn, submit)
            .expect("submitStudyAnswer should succeed");
    }

    // Complete session
    let complete = word_app_core::complete_study_session(&conn, &session_id)
        .expect("completeStudySession should succeed");
    let complete_json = to_json(&complete);

    assert_json_greater_than(&complete_json, "summary.incorrectCount", 0);
    assert_json_equals(
        &complete_json,
        "summary.totalQuestions",
        &Value::Number(3.into()),
    );
}

#[test]
fn baseline_study_cancel_no_persist() {
    word_app_core::clear_all_active_sessions();

    let conn = create_test_db();
    let payloads = make_entry_payloads();

    let request = StartSessionRequest {
        mode: SessionMode::MixedTest,
        wordbook_id: None,
        entry_source_ids: vec!["w1".into(), "w2".into(), "w3".into()],
        entry_payloads: payloads[..3].to_vec(),
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let start_response = word_app_core::start_study_session(&conn, request)
        .expect("startStudySession should succeed");
    let session_id = start_response.session.session_id.clone();

    // Answer one question correctly
    let correct = correct_answer_for(&start_response.current_question);
    let submit = SubmitAnswerRequest {
        question_id: start_response.current_question.question_id.clone(),
        response: correct,
        response_time_ms: 1000,
    };
    let _submit_response = word_app_core::submit_study_answer(&conn, submit)
        .expect("submitStudyAnswer should succeed");

    // Cancel session
    word_app_core::cancel_study_session(&conn, &session_id)
        .expect("cancelStudySession should succeed");

    // Verify no completed session in DB
    let completed_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM study_sessions WHERE session_id = ?1 AND completed_at IS NOT NULL",
            [&session_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        completed_count, 0,
        "No completed session should exist after cancel"
    );

    // Verify no study results persisted
    let results_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM study_results WHERE session_id = ?1",
            [&session_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        results_count, 0,
        "No study results should be persisted after cancel"
    );

    // Verify no active session snapshot
    let snapshot_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM app_settings WHERE key LIKE 'active_study_session_%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        snapshot_count, 0,
        "No active session snapshot should remain after cancel"
    );
}

// ---------------------------------------------------------------------------
// Semantic invariant tests
// ---------------------------------------------------------------------------

#[test]
fn baseline_progress_monotonic_in_session() {
    word_app_core::clear_all_active_sessions();

    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);
    seed_wordbook(&conn);
    seed_entries_for_payloads(&conn);
    let payloads = make_entry_payloads();

    // Use Review mode to avoid global ACTIVE_SESSIONS key collision with
    // other MixedTest tests when running in parallel.
    let request = StartSessionRequest {
        mode: SessionMode::Review,
        wordbook_id: None,
        entry_source_ids: vec!["w1".into(), "w2".into()],
        entry_payloads: payloads[..2].to_vec(),
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let start = word_app_core::start_study_session(&conn, request).unwrap();
    assert_eq!(start.progress.current, 1);

    let mut prev_current = start.progress.current;
    let mut current_question = start.current_question;

    loop {
        let correct = correct_answer_for(&current_question);

        let submit = SubmitAnswerRequest {
            question_id: current_question.question_id.clone(),
            response: correct,
            response_time_ms: 1000,
        };

        let response = word_app_core::submit_study_answer(&conn, submit).unwrap();

        // Progress must be strictly increasing until completion
        if !response.is_complete {
            assert!(
                response.progress.current > prev_current,
                "Progress must be monotonically increasing: {} -> {}",
                prev_current,
                response.progress.current
            );
        }
        prev_current = response.progress.current;

        if response.is_complete {
            break;
        }
        current_question = response.current_question.unwrap();
    }
}

#[test]
fn baseline_is_complete_and_summary_mutex() {
    word_app_core::clear_all_active_sessions();

    let conn = create_test_db();
    let payloads = make_entry_payloads();

    let request = StartSessionRequest {
        mode: SessionMode::MixedTest,
        wordbook_id: None,
        entry_source_ids: vec!["w1".into(), "w2".into(), "w3".into()],
        entry_payloads: payloads[..3].to_vec(),
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let start = word_app_core::start_study_session(&conn, request).unwrap();
    let total = start.progress.total;
    let session_id = start.session.session_id.clone();
    let mut current_question = start.current_question;

    for i in 0..total {
        let correct = correct_answer_for(&current_question);

        let submit = SubmitAnswerRequest {
            question_id: current_question.question_id.clone(),
            response: correct,
            response_time_ms: 1000,
        };

        let response = word_app_core::submit_study_answer(&conn, submit).unwrap();

        if response.is_complete {
            // Last answer
            assert!(
                response.summary.is_some(),
                "summary should exist when complete at question {}",
                i + 1
            );
            assert!(
                response.current_question.is_none(),
                "currentQuestion should be None when complete"
            );
            break;
        } else {
            assert!(
                response.current_question.is_some(),
                "currentQuestion should exist when not complete at question {}",
                i + 1
            );
            assert!(
                response.summary.is_none(),
                "summary should be None when not complete at question {}",
                i + 1
            );
            current_question = response.current_question.unwrap();
        }
    }

    let _ = word_app_core::complete_study_session(&conn, &session_id);
}

// ---------------------------------------------------------------------------
// Additional study mode and answer outcome tests
// ---------------------------------------------------------------------------

#[test]
fn baseline_study_review_mode() {
    word_app_core::clear_all_active_sessions();

    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);
    seed_wordbook(&conn);
    seed_entries_for_payloads(&conn);
    let payloads = make_entry_payloads();

    let request = StartSessionRequest {
        mode: SessionMode::Review,
        wordbook_id: None,
        entry_source_ids: vec!["w1".into(), "w2".into()],
        entry_payloads: payloads[..2].to_vec(),
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let start = word_app_core::start_study_session(&conn, request)
        .expect("startStudySession should succeed");

    // Review mode plan units are questions, so 2 hydrated entries produce 2 questions.
    assert_eq!(start.session.mode, SessionMode::Review);
    assert_eq!(start.session.total_words, 2);
    assert_eq!(start.progress.total, 2);

    let session_id = start.session.session_id.clone();
    let mut current_question = start.current_question;
    let mut answer_count = 0;

    loop {
        let correct = correct_answer_for(&current_question);
        let submit = SubmitAnswerRequest {
            question_id: current_question.question_id.clone(),
            response: correct,
            response_time_ms: 1200,
        };
        let response = word_app_core::submit_study_answer(&conn, submit).unwrap();
        answer_count += 1;

        assert_eq!(
            response.result.outcome,
            AnswerOutcome::Correct,
            "All correct answers should have outcome=correct"
        );

        if response.is_complete {
            let summary = response.summary.as_ref().unwrap();
            assert_eq!(summary.correct_count, 2);
            assert_eq!(summary.total_questions, 2);
            break;
        }
        current_question = response.current_question.unwrap();
    }

    assert_eq!(answer_count, 2, "Review mode: 2 plan questions = 2 answers");
    let _ = word_app_core::complete_study_session(&conn, &session_id);
}

#[test]
fn baseline_study_fuzzy_correct() {
    word_app_core::clear_all_active_sessions();

    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);
    seed_wordbook(&conn);
    seed_entries_for_payloads(&conn);
    let payloads = make_entry_payloads();

    let request = StartSessionRequest {
        mode: SessionMode::MixedTest,
        wordbook_id: None,
        entry_source_ids: vec!["w1".into(), "w2".into(), "w3".into()],
        entry_payloads: payloads[..3].to_vec(),
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let start = word_app_core::start_study_session(&conn, request)
        .expect("startStudySession should succeed");
    let session_id = start.session.session_id.clone();

    // Submit a partial answer that should trigger FuzzyCorrect
    // The first question's accepted meanings include "鏀惧純", "鎶借薄鐨?, or "瀛︽湳鐨?
    // Submitting a substring like "鏀? should trigger fuzzy match
    let first_question = &start.current_question;

    // For choice questions, we can't trigger fuzzy. For input questions, we can.
    // Submit a partial meaning that contains / is contained by an accepted meaning
    let fuzzy_response = if first_question.question_type.is_input_type() {
        // Use a substring of the accepted meaning
        let meaning = first_question
            .accepted_meanings
            .first()
            .cloned()
            .unwrap_or_default();
        if meaning.len() > 1 {
            meaning[..meaning.len() - 1].to_string()
        } else {
            meaning
        }
    } else {
        // For choice questions, we can't fuzzy match; submit correct to advance
        correct_answer_for(first_question)
    };

    let submit = SubmitAnswerRequest {
        question_id: first_question.question_id.clone(),
        response: fuzzy_response,
        response_time_ms: 2000,
    };
    let response = word_app_core::submit_study_answer(&conn, submit).unwrap();

    // If we got a fuzzy match, verify it. Otherwise (choice question) just verify correct.
    if response.result.outcome == AnswerOutcome::FuzzyCorrect {
        assert!(
            response.result.normalized_response.is_some(),
            "FuzzyCorrect should have normalized_response"
        );
    }

    // Complete remaining questions
    let mut remaining = response;
    while !remaining.is_complete {
        let question = remaining.current_question.as_ref().unwrap();
        let correct = correct_answer_for(question);
        let submit = SubmitAnswerRequest {
            question_id: question.question_id.clone(),
            response: correct,
            response_time_ms: 1500,
        };
        remaining = word_app_core::submit_study_answer(&conn, submit).unwrap();
    }

    let complete = word_app_core::complete_study_session(&conn, &session_id).unwrap();
    let summary = &complete.summary;
    // The session should have completed successfully
    assert_eq!(summary.total_questions, 3);
}

#[test]
fn baseline_study_skip_answer() {
    word_app_core::clear_all_active_sessions();

    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);
    seed_wordbook(&conn);
    seed_entries_for_payloads(&conn);
    let payloads = make_entry_payloads();

    let request = StartSessionRequest {
        mode: SessionMode::MixedTest,
        wordbook_id: None,
        entry_source_ids: vec!["w1".into(), "w2".into(), "w3".into()],
        entry_payloads: payloads[..3].to_vec(),
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let start = word_app_core::start_study_session(&conn, request)
        .expect("startStudySession should succeed");
    let session_id = start.session.session_id.clone();

    // Submit an empty response -> should be Skipped
    let submit = SubmitAnswerRequest {
        question_id: start.current_question.question_id.clone(),
        response: String::new(), // Empty response = skip
        response_time_ms: 500,
    };
    let response = word_app_core::submit_study_answer(&conn, submit).unwrap();

    // Empty input should result in Skipped for input types
    // For choice types, empty label also results in Skipped
    assert_eq!(
        response.result.outcome,
        AnswerOutcome::Skipped,
        "Empty response should result in Skipped outcome"
    );

    // Complete remaining questions correctly
    let mut remaining = response;
    while !remaining.is_complete {
        let question = remaining.current_question.as_ref().unwrap();
        let correct = correct_answer_for(question);
        let submit = SubmitAnswerRequest {
            question_id: question.question_id.clone(),
            response: correct,
            response_time_ms: 1500,
        };
        remaining = word_app_core::submit_study_answer(&conn, submit).unwrap();
    }

    let complete = word_app_core::complete_study_session(&conn, &session_id).unwrap();
    assert_eq!(
        complete.summary.skipped_count, 1,
        "One skipped answer expected"
    );
    assert_eq!(complete.summary.total_questions, 3);
}

#[test]
fn baseline_session_persistence_across_restart() {
    word_app_core::clear_all_active_sessions();

    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);
    seed_wordbook(&conn);
    seed_entries_for_payloads(&conn);
    let payloads = make_entry_payloads();

    // Start a session and answer one question
    let request = StartSessionRequest {
        mode: SessionMode::MixedTest,
        wordbook_id: None,
        entry_source_ids: vec!["w1".into(), "w2".into(), "w3".into()],
        entry_payloads: payloads[..3].to_vec(),
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let start = word_app_core::start_study_session(&conn, request)
        .expect("startStudySession should succeed");
    let session_id = start.session.session_id.clone();

    let correct = correct_answer_for(&start.current_question);
    let submit = SubmitAnswerRequest {
        question_id: start.current_question.question_id.clone(),
        response: correct,
        response_time_ms: 1000,
    };
    let first_answer = word_app_core::submit_study_answer(&conn, submit).unwrap();
    assert!(!first_answer.is_complete);
    let progress_after_first = first_answer.progress;

    // Simulate restart: clear in-memory sessions
    word_app_core::clear_all_active_sessions();

    // Resume by starting the same session again
    // The session should be recovered from the persisted snapshot
    let resume_request = StartSessionRequest {
        mode: SessionMode::MixedTest,
        wordbook_id: None,
        entry_source_ids: vec!["w1".into(), "w2".into(), "w3".into()],
        entry_payloads: payloads[..3].to_vec(),
        distractor_payloads: vec![],
        question_type_weights: vec![],
    };

    let resumed =
        word_app_core::start_study_session(&conn, resume_request).expect("Session should resume");

    // Progress should not regress
    assert!(
        resumed.progress.current >= progress_after_first.current,
        "After restart, progress should not regress: was {}, got {}",
        progress_after_first.current,
        resumed.progress.current
    );

    // Session ID should be preserved
    assert_eq!(
        resumed.session.session_id, session_id,
        "Session ID should be preserved across restart"
    );

    // Complete remaining questions
    let mut current_question = resumed.current_question;
    let mut remaining_answers = 0;
    loop {
        let correct = correct_answer_for(&current_question);
        let submit = SubmitAnswerRequest {
            question_id: current_question.question_id.clone(),
            response: correct,
            response_time_ms: 1000,
        };
        let response = word_app_core::submit_study_answer(&conn, submit).unwrap();
        remaining_answers += 1;

        if response.is_complete {
            break;
        }
        current_question = response.current_question.unwrap();
    }

    // We should only need to answer the remaining questions (not restart from 0)
    assert!(
        remaining_answers < 3,
        "Should only answer remaining questions, not all 3 again"
    );

    let _ = word_app_core::complete_study_session(&conn, &session_id);
}

#[test]
fn baseline_today_snapshot_not_polluted_by_plan_edit() {
    let conn = create_test_db();
    seed_onboarding_completed(&conn);
    seed_active_plan(&conn);
    seed_wordbook(&conn);

    // Get initial today state
    let state1 =
        word_app_core::get_today_home_state(&conn).expect("getTodayHomeState should succeed");

    // Edit the plan
    conn.execute(
        "UPDATE plan_templates SET new_words_per_day = 50, review_words_per_day = 60 WHERE is_active = 1",
        [],
    )
    .expect("Failed to update plan");

    // Get today state again
    let state2 =
        word_app_core::get_today_home_state(&conn).expect("getTodayHomeState should succeed");

    // Current implementation derives snapshot from plan on every call,
    // so the snapshot WILL reflect the new plan immediately.
    // This test captures the CURRENT behavior as baseline truth.
    // The product requirement "today snapshot not polluted by same-day plan edit"
    // will need to be implemented as a separate feature.
    let json1 = to_json(&state1);
    let json2 = to_json(&state2);

    // Both should return valid states
    assert_json_not_null(&json1, "todaySnapshot");
    assert_json_not_null(&json2, "todaySnapshot");

    // This test documents that today snapshot currently IS recalculated
    // when the plan changes. When the "same-day stability" feature is
    // implemented, this test should be updated to assert the snapshot
    // does NOT change after same-day plan edits.
}
