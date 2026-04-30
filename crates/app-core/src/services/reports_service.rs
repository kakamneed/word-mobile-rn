//! Reports aggregation service.
//!
//! Extracted from `platform-mobile/bridge.rs` to make report computation
//! testable from app-core without depending on the cdylib bridge layer.

use std::collections::BTreeMap;

use serde_json::Value;

/// Reports overview computed from session history.
///
/// This is the domain logic previously embedded in `bridge.rs::build_reports_overview()`.
/// It takes raw session history items and produces the full reports overview.
pub fn build_reports_overview(today_date: &str, history: &[Value], learned_count: u64) -> Value {
    let mut by_date = BTreeMap::<String, Value>::new();
    let mut by_mode = BTreeMap::<String, Value>::new();
    let mut by_mode_date = BTreeMap::<String, BTreeMap<String, Value>>::new();
    let mut study_days = std::collections::BTreeSet::<String>::new();
    let mut total_questions = 0.0f64;
    let mut total_correct = 0.0f64;

    for item in history {
        let date = item
            .get("date")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mode = item
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mode = normalize_enum_text(&mode);
        let Some(summary) = item.get("summary").and_then(|v| v.as_object()) else {
            continue;
        };
        study_days.insert(date.clone());
        total_questions += summary
            .get("totalQuestions")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        total_correct += summary
            .get("correctCount")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let total_q = summary
            .get("totalQuestions")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let correct = summary
            .get("correctCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let time_ms = summary
            .get("totalTimeMs")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Aggregate by date
        let day = by_date.entry(date.clone()).or_insert_with(|| {
            serde_json::json!({
                "date": &date,
                "totalQuestions": 0,
                "correctCount": 0,
                "studyTimeMs": 0
            })
        });
        day["totalQuestions"] = serde_json::json!(
            day.get("totalQuestions")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + total_q
        );
        day["correctCount"] = serde_json::json!(
            day.get("correctCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + correct
        );
        day["studyTimeMs"] = serde_json::json!(
            day.get("studyTimeMs").and_then(|v| v.as_u64()).unwrap_or(0) + time_ms
        );

        // Aggregate by mode
        let mode_entry = by_mode.entry(mode.clone()).or_insert_with(|| {
            serde_json::json!({
                "totalQuestions": 0,
                "correctCount": 0
            })
        });
        mode_entry["totalQuestions"] = serde_json::json!(
            mode_entry
                .get("totalQuestions")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + total_q
        );
        mode_entry["correctCount"] = serde_json::json!(
            mode_entry
                .get("correctCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + correct
        );

        // Aggregate by mode+date
        let mode_date_map = by_mode_date.entry(mode.clone()).or_default();
        let mode_date_entry = mode_date_map.entry(date.clone()).or_insert_with(|| {
            serde_json::json!({
                "date": &date,
                "totalQuestions": 0,
                "correctCount": 0,
                "studyTimeMs": 0
            })
        });
        mode_date_entry["totalQuestions"] = serde_json::json!(
            mode_date_entry
                .get("totalQuestions")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + total_q
        );
        mode_date_entry["correctCount"] = serde_json::json!(
            mode_date_entry
                .get("correctCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + correct
        );
        mode_date_entry["studyTimeMs"] = serde_json::json!(
            mode_date_entry
                .get("studyTimeMs")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                + time_ms
        );
    }

    let overall_accuracy = if total_questions > 0.0 {
        (total_correct / total_questions) * 100.0
    } else {
        0.0
    };

    let last7 = build_recent_days(&by_date, today_date, 7);
    let daily_series = build_all_days_series(&by_date);
    let mode_breakdown = build_mode_breakdown(&by_mode);
    let mode_series = build_mode_series(&by_mode_date);
    let streak = build_streak_info(&study_days, today_date);

    serde_json::json!({
        "totalStudyDays": study_days.len(),
        "totalWordsLearned": learned_count,
        "totalQuestionsAnswered": total_questions as u64,
        "overallAccuracy": overall_accuracy,
        "streakInfo": streak,
        "modeBreakdown": mode_breakdown,
        "last7Days": last7,
        "dailySeries": daily_series,
        "modeSeries": mode_series
    })
}

fn normalize_enum_text(value: &str) -> String {
    serde_json::from_str::<String>(value).unwrap_or_else(|_| value.to_string())
}

/// Build recent N days of report data, filling gaps with zero entries.
pub fn build_recent_days(
    by_date: &BTreeMap<String, Value>,
    today_date: &str,
    days: usize,
) -> Vec<Value> {
    let end = chrono::NaiveDate::parse_from_str(today_date, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Local::now().date_naive());
    let mut result = Vec::new();
    for offset in (0..days).rev() {
        let date = end - chrono::Days::new(offset as u64);
        let key = date.format("%Y-%m-%d").to_string();
        let day = by_date.get(&key);
        let total_questions = day
            .and_then(|v| v.get("totalQuestions"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let correct_count = day
            .and_then(|v| v.get("correctCount"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let study_time_ms = day
            .and_then(|v| v.get("studyTimeMs"))
            .and_then(|v| v.as_u64())
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

/// Build full daily time series from aggregated date data.
pub fn build_all_days_series(by_date: &BTreeMap<String, Value>) -> Vec<Value> {
    by_date
        .iter()
        .map(|(date, day)| {
            let total_questions = day.get("totalQuestions").and_then(|v| v.as_u64()).unwrap_or(0);
            let correct_count = day.get("correctCount").and_then(|v| v.as_u64()).unwrap_or(0);
            let study_time_ms = day.get("studyTimeMs").and_then(|v| v.as_u64()).unwrap_or(0);
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

/// Build per-mode breakdown.
pub fn build_mode_breakdown(by_mode: &BTreeMap<String, Value>) -> Vec<Value> {
    ["newWord", "review", "mixedTest", "wrongWordReinforcement", "rootAffix"]
        .iter()
        .map(|mode| {
            let mode_total = by_mode.get(*mode);
            let total_questions = mode_total
                .and_then(|v| v.get("totalQuestions"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let correct_count = mode_total
                .and_then(|v| v.get("correctCount"))
                .and_then(|v| v.as_u64())
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

/// Build per-mode daily time series.
pub fn build_mode_series(by_mode_date: &BTreeMap<String, BTreeMap<String, Value>>) -> Value {
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
            .map(|m| build_all_days_series(m))
            .unwrap_or_default();
        out.insert(mode.to_string(), Value::Array(series));
    }
    Value::Object(out)
}

/// Build streak information from study days.
pub fn build_streak_info(
    study_days: &std::collections::BTreeSet<String>,
    today_date: &str,
) -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_reports_overview_empty() {
        let result = build_reports_overview("2026-04-22", &[], 0);
        assert_eq!(result["totalStudyDays"], 0);
        assert_eq!(result["totalQuestionsAnswered"], 0);
        assert_eq!(result["overallAccuracy"], 0.0);
    }

    #[test]
    fn test_reports_overview_with_history() {
        let history = vec![json!({
            "date": "2026-04-22",
            "mode": "newWord",
            "summary": {
                "totalQuestions": 20,
                "correctCount": 18,
                "totalTimeMs": 30000
            }
        })];

        let result = build_reports_overview("2026-04-22", &history, 5);
        assert_eq!(result["totalStudyDays"], 1);
        assert_eq!(result["totalQuestionsAnswered"], 20);
        assert_eq!(result["totalWordsLearned"], 5);

        let accuracy = result["overallAccuracy"].as_f64().unwrap();
        assert!((accuracy - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_streak_info_consecutive() {
        let mut days = std::collections::BTreeSet::new();
        days.insert("2026-04-20".to_string());
        days.insert("2026-04-21".to_string());
        days.insert("2026-04-22".to_string());

        let result = build_streak_info(&days, "2026-04-22");
        assert_eq!(result["currentStreak"], 3);
        assert_eq!(result["longestStreak"], 3);
    }

    #[test]
    fn test_mode_breakdown() {
        let mut by_mode = BTreeMap::new();
        by_mode.insert(
            "newWord".to_string(),
            json!({"totalQuestions": 20, "correctCount": 18}),
        );

        let result = build_mode_breakdown(&by_mode);
        assert_eq!(result.len(), 5);

        let new_word = result.iter().find(|v| v["mode"] == "newWord").unwrap();
        assert_eq!(new_word["totalQuestions"], 20);
        assert_eq!(new_word["correctCount"], 18);
    }

    #[test]
    fn test_reports_overview_normalizes_json_encoded_modes() {
        let history = vec![json!({
            "date": "2026-04-22",
            "mode": "\"mixedTest\"",
            "summary": {
                "totalQuestions": 10,
                "correctCount": 7,
                "totalTimeMs": 12000
            }
        })];

        let result = build_reports_overview("2026-04-22", &history, 0);
        let mixed = result["modeBreakdown"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["mode"] == "mixedTest")
            .unwrap();

        assert_eq!(mixed["totalQuestions"], 10);
        assert_eq!(mixed["correctCount"], 7);
        assert_eq!(result["modeSeries"]["mixedTest"][0]["totalQuestions"], 10);
    }
}
