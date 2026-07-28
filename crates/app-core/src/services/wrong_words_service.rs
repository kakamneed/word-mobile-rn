//! Wrong-word computation service.
//!
//! Extracted from `platform-mobile/bridge.rs` to make wrong-word scoring,
//! sorting, and filtering testable from app-core without depending on the
//! cdylib bridge layer.

use std::collections::BTreeMap;

use serde_json::Value;

/// Compute priority scores for wrong-word entries and sort/filter them.
///
/// This is the domain logic previously embedded in `bridge.rs::build_wrong_words()`.
/// It takes a list of entries and a filter, computes priority scores, and returns
/// the sorted/filtered result.
pub fn build_wrong_words(entries: &mut [Value], filter: &str) -> Vec<Value> {
    let local_day = chrono::Local::now().date_naive().to_string();
    build_wrong_words_at(entries, filter, &local_day)
}

pub fn build_wrong_words_at(entries: &mut [Value], filter: &str, local_day: &str) -> Vec<Value> {
    let context = word_domain_core::DomainContext {
        now_utc: String::new(),
        local_day: local_day.to_string(),
        session_id: String::new(),
        ordering_seed: String::new(),
    };
    for entry in entries.iter_mut() {
        apply_priority_fields(entry, &context);
        if entry.get("isActive").is_none() {
            entry["isActive"] = serde_json::json!(true);
        }
    }

    let mut merged_by_entry_key = BTreeMap::<(String, String), Value>::new();
    for entry in entries.iter().filter(|entry| {
        entry
            .get("errorCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            > 0
    }) {
        let word_key = entry
            .get("word")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if word_key.is_empty() {
            continue;
        }
        let entry_kind = entry
            .get("entryKind")
            .and_then(|v| v.as_str())
            .unwrap_or("word")
            .to_string();
        let entry_key = (entry_kind, word_key);

        let Some(existing) = merged_by_entry_key.get_mut(&entry_key) else {
            merged_by_entry_key.insert(entry_key, entry.clone());
            continue;
        };

        let existing_errors = existing
            .get("errorCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let entry_errors = entry
            .get("errorCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let total_errors = existing_errors + entry_errors;
        existing["errorCount"] = serde_json::json!(total_errors);

        let existing_last_wrong = existing
            .get("lastWrongAt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let entry_last_wrong = entry
            .get("lastWrongAt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if entry_last_wrong > existing_last_wrong {
            existing["lastWrongAt"] = serde_json::json!(entry_last_wrong);
        }
        merge_numeric_field(existing, entry, "correctCount");
        let existing_last_correct = existing
            .get("lastCorrectAt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let entry_last_correct = entry
            .get("lastCorrectAt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if entry_last_correct > existing_last_correct {
            existing["lastCorrectAt"] = serde_json::json!(entry_last_correct);
        }

        apply_priority_fields(existing, &context);
        merge_meanings(existing, entry);
    }

    let mut result: Vec<Value> = merged_by_entry_key.into_values().collect();
    result.sort_by(|left, right| compare_wrong_word_entries(left, right, filter));

    if filter == "highPriority" {
        result.retain(|entry| {
            entry
                .get("priorityScore")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                >= 8.0
        });
    }

    result
}

fn apply_priority_fields(entry: &mut Value, context: &word_domain_core::DomainContext) {
    let error_count = numeric_field(entry, "errorCount");
    let correct_count = numeric_field(entry, "correctCount");
    let last_wrong_at = entry
        .get("lastWrongAt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let correct_since_last_wrong = if entry.get("correctSinceLastWrong").is_some() {
        numeric_field(entry, "correctSinceLastWrong")
    } else {
        corrects_after_last_wrong(entry)
    };
    entry["correctSinceLastWrong"] = serde_json::json!(correct_since_last_wrong as i64);

    let input = word_domain_core::WrongWordProjectionInput {
        entry_source_id: entry
            .get("entrySourceId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        word: entry
            .get("word")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        error_count: error_count.max(0.0) as u64,
        correct_count: correct_count.max(0.0) as u64,
        consecutive_correct: entry
            .get("consecutiveCorrect")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        last_wrong_at,
        correct_since_last_wrong: correct_since_last_wrong.max(0.0) as u64,
        is_active: entry.get("isActive").and_then(Value::as_bool).unwrap_or(true),
    };
    let projection = word_domain_core::score_wrong_word(context, &input);
    entry["errorRate"] = serde_json::json!(projection.error_rate);
    entry["priorityScore"] = serde_json::json!(projection.priority_score);
}

fn corrects_after_last_wrong(entry: &Value) -> f64 {
    let last_wrong_at = entry
        .get("lastWrongAt")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let last_correct_at = entry
        .get("lastCorrectAt")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !last_wrong_at.is_empty() && last_correct_at > last_wrong_at {
        numeric_field(entry, "correctCount").max(1.0)
    } else {
        0.0
    }
}

fn numeric_field(entry: &Value, field: &str) -> f64 {
    entry.get(field).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

fn merge_numeric_field(existing: &mut Value, duplicate: &Value, field: &str) {
    let total = existing.get(field).and_then(|v| v.as_i64()).unwrap_or(0)
        + duplicate.get(field).and_then(|v| v.as_i64()).unwrap_or(0);
    existing[field] = serde_json::json!(total);
}

fn merge_meanings(existing: &mut Value, duplicate: &Value) {
    let Some(existing_meanings) = existing.get_mut("meanings").and_then(|v| v.as_array_mut())
    else {
        return;
    };
    let Some(duplicate_meanings) = duplicate.get("meanings").and_then(|v| v.as_array()) else {
        return;
    };
    for meaning in duplicate_meanings {
        if !existing_meanings.iter().any(|item| item == meaning) {
            existing_meanings.push(meaning.clone());
        }
    }
}

/// Recency score for wrong-word priority calculation.
///
/// Returns a value 0-21 based on how recently the word was answered wrong.
/// Recent errors score higher.
pub fn recency_score(last_wrong_at: &str) -> f64 {
    let date = last_wrong_at.get(0..10).unwrap_or(last_wrong_at);
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        let today = chrono::Local::now().date_naive();
        let days = (today - parsed).num_days().max(0);
        (21 - days.min(21)) as f64
    } else {
        0.0
    }
}

/// Recency score with a fixed "today" date for deterministic testing.
pub fn recency_score_at(last_wrong_at: &str, today: chrono::NaiveDate) -> f64 {
    let date = last_wrong_at.get(0..10).unwrap_or(last_wrong_at);
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        let days = (today - parsed).num_days().max(0);
        (21 - days.min(21)) as f64
    } else {
        0.0
    }
}

fn compare_wrong_word_entries(left: &Value, right: &Value, filter: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let left_active = left
        .get("isActive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let right_active = right
        .get("isActive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    match (left_active, right_active) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }

    if filter == "recent" {
        let left_date = left
            .get("lastWrongAt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let right_date = right
            .get("lastWrongAt")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        return right_date.cmp(left_date);
    }

    let left_priority = left
        .get("priorityScore")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let right_priority = right
        .get("priorityScore")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    right_priority
        .partial_cmp(&left_priority)
        .unwrap_or(Ordering::Equal)
        .then_with(|| {
            right
                .get("errorCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                .cmp(&left.get("errorCount").and_then(|v| v.as_u64()).unwrap_or(0))
        })
        .then_with(|| {
            right
                .get("lastWrongAt")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .cmp(
                    left.get("lastWrongAt")
                        .and_then(|v| v.as_str())
                        .unwrap_or(""),
                )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_wrong_word_priority_scoring() {
        let mut entries = vec![json!({
            "entryId": "w1",
            "word": "abandon",
            "errorCount": 3,
            "lastWrongAt": "2026-04-20",
            "isActive": true
        })];

        let result = build_wrong_words(&mut entries, "all");
        assert_eq!(result.len(), 1);
        let priority = result[0]
            .get("priorityScore")
            .and_then(|v| v.as_f64())
            .unwrap();
        assert!(priority > 0.0, "Priority should be positive");
        assert!(priority <= 10.0, "Priority should be capped at 10.0");
        assert!(result[0].get("errorRate").is_some());
        assert!(result[0].get("correctSinceLastWrong").is_some());
    }

    #[test]
    fn test_recency_score_deterministic() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 4, 22).unwrap();
        // Same day: recency should be 21
        assert_eq!(recency_score_at("2026-04-22", today), 21.0);
        // 21 days ago: recency should be 0
        assert_eq!(recency_score_at("2026-04-01", today), 0.0);
        // 10 days ago: recency should be 11
        assert_eq!(recency_score_at("2026-04-12", today), 11.0);
    }

    #[test]
    fn test_high_priority_filter() {
        let mut entries = vec![
            json!({
                "entryId": "w1",
                "word": "abandon",
                "errorCount": 10,
                "lastWrongAt": "2026-04-22",
            }),
            json!({
                "entryId": "w2",
                "word": "accept",
                "errorCount": 1,
                "lastWrongAt": "2026-04-01",
            }),
        ];

        let result = build_wrong_words(&mut entries, "highPriority");
        assert!(result.len() <= 2);
        for entry in &result {
            let p = entry.get("priorityScore").and_then(|v| v.as_f64()).unwrap();
            assert!(p >= 8.0, "Only high priority entries should remain");
        }
    }

    #[test]
    fn test_zero_error_entries_are_hidden_from_wrong_word_list() {
        let mut entries = vec![
            json!({
                "entryId": "w1",
                "word": "learned_only",
                "errorCount": 0,
                "lastWrongAt": "",
            }),
            json!({
                "entryId": "w2",
                "word": "missed",
                "errorCount": 1,
                "lastWrongAt": "2026-04-22",
            }),
        ];

        let result = build_wrong_words(&mut entries, "all");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["word"], "missed");
    }

    #[test]
    fn test_duplicate_word_entries_are_merged_for_display() {
        let mut entries = vec![
            json!({
                "entryId": 1,
                "word": "cancel",
                "meanings": ["相互抵消"],
                "errorCount": 4,
                "lastWrongAt": "2026-04-29T03:00:00Z",
            }),
            json!({
                "entryId": 2,
                "word": "Cancel",
                "meanings": ["取消，撤销，删去"],
                "errorCount": 2,
                "lastWrongAt": "2026-04-29T03:06:00Z",
            }),
        ];

        let result = build_wrong_words(&mut entries, "all");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["word"], "cancel");
        assert_eq!(result[0]["errorCount"], 6);
        assert_eq!(result[0]["lastWrongAt"], "2026-04-29T03:06:00Z");
        assert_eq!(result[0]["meanings"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_word_and_root_affix_entries_with_same_text_stay_separate() {
        let mut entries = vec![
            json!({
                "entryId": 1,
                "word": "abs",
                "entryKind": "rootAffix",
                "meanings": [],
                "errorCount": 1,
                "lastWrongAt": "2026-04-29T03:00:00Z",
            }),
            json!({
                "entryId": 2,
                "word": "Abs",
                "entryKind": "word",
                "meanings": ["abs meaning"],
                "errorCount": 1,
                "lastWrongAt": "2026-04-29T03:06:00Z",
            }),
        ];

        let result = build_wrong_words(&mut entries, "all");

        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|entry| entry["entryKind"] == "rootAffix"));
        assert!(result.iter().any(|entry| entry["entryKind"] == "word"));
    }

    #[test]
    fn consecutive_correct_answers_lower_wrong_word_priority() {
        let mut entries = vec![
            json!({
                "entryId": 1,
                "word": "recovered",
                "errorCount": 5,
                "correctCount": 4,
                "correctSinceLastWrong": 3,
                "lastWrongAt": "2026-04-22T01:00:00Z",
                "lastCorrectAt": "2026-04-22T01:30:00Z",
            }),
            json!({
                "entryId": 2,
                "word": "still_wrong",
                "errorCount": 3,
                "correctCount": 0,
                "correctSinceLastWrong": 0,
                "lastWrongAt": "2026-04-22T01:10:00Z",
            }),
        ];

        let result = build_wrong_words(&mut entries, "all");

        assert_eq!(result[0]["word"], "still_wrong");
        let recovered_priority = result
            .iter()
            .find(|entry| entry["word"] == "recovered")
            .and_then(|entry| entry.get("priorityScore"))
            .and_then(|value| value.as_f64())
            .unwrap();
        let still_wrong_priority = result
            .iter()
            .find(|entry| entry["word"] == "still_wrong")
            .and_then(|entry| entry.get("priorityScore"))
            .and_then(|value| value.as_f64())
            .unwrap();
        assert!(recovered_priority < still_wrong_priority);
    }

    #[test]
    fn wrong_words_service_matches_direct_domain_projection_at_fixed_day() {
        let mut entries = vec![json!({
            "entrySourceId": "entry-alpha",
            "word": "alpha",
            "errorCount": 3,
            "correctCount": 1,
            "consecutiveCorrect": 0,
            "lastWrongAt": "fixed-fixture-date",
            "correctSinceLastWrong": 0,
            "isActive": true
        })];
        let context = word_domain_core::DomainContext {
            now_utc: "2026-07-28T00:00:00Z".to_string(),
            local_day: "2026-07-28".to_string(),
            session_id: "wrong-equivalence".to_string(),
            ordering_seed: "wrong-equivalence".to_string(),
        };
        let typed = vec![serde_json::from_value(entries[0].clone()).unwrap()];

        assert_eq!(
            serde_json::Value::Array(build_wrong_words_at(
                &mut entries,
                "all",
                &context.local_day,
            )),
            serde_json::to_value(word_domain_core::project_wrong_words(
                &context, &typed, "all"
            ))
            .unwrap()
        );
    }
}
