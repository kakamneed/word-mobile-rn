//! Native JSON adapter for deterministic report projections.

use serde_json::Value;
use word_domain_core::{project_report, DomainContext, ReportHistoryInput};

/// Convert repository-shaped JSON history into the typed pure report projection.
pub fn build_reports_overview(today_date: &str, history: &[Value], learned_count: u64) -> Value {
    let context = DomainContext {
        now_utc: String::new(),
        local_day: today_date.to_string(),
        session_id: String::new(),
        ordering_seed: String::new(),
    };
    let typed_history = history
        .iter()
        .filter_map(|item| serde_json::from_value(item.clone()).ok())
        .collect::<Vec<ReportHistoryInput>>();
    serde_json::to_value(project_report(&context, &typed_history, learned_count))
        .expect("typed report projection serializes")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reports_service_matches_direct_domain_projection() {
        let history = vec![
            json!({
                "date": "2026-04-30",
                "mode": "\"review\"",
                "summary": { "totalQuestions": 2, "correctCount": 1, "totalTimeMs": 1000 }
            }),
            json!({
                "date": "2026-05-01",
                "mode": "mixedTest",
                "summary": { "totalQuestions": 1, "correctCount": 1, "totalTimeMs": 600 }
            }),
            json!({
                "date": "2026-05-01",
                "mode": "newWord",
                "summary": { "totalQuestions": 0, "correctCount": 0, "totalTimeMs": 0 }
            }),
        ];
        let context = DomainContext {
            now_utc: "2026-05-01T12:00:00Z".to_string(),
            local_day: "2026-05-01".to_string(),
            session_id: "report-equivalence".to_string(),
            ordering_seed: "report-equivalence".to_string(),
        };
        let typed = history
            .iter()
            .cloned()
            .map(serde_json::from_value)
            .collect::<Result<Vec<ReportHistoryInput>, _>>()
            .unwrap();

        assert_eq!(
            build_reports_overview(&context.local_day, &history, 12),
            serde_json::to_value(project_report(&context, &typed, 12)).unwrap()
        );
    }

    #[test]
    fn reports_service_filters_zero_answer_rows_and_fills_from_supplied_day() {
        let history = vec![json!({
            "date": "2026-05-01",
            "mode": "review",
            "summary": { "totalQuestions": 0, "correctCount": 0, "totalTimeMs": 0 }
        })];

        let report = build_reports_overview("2026-05-01", &history, 0);
        assert_eq!(report["totalStudyDays"], 0);
        assert_eq!(report["last7Days"][6]["date"], "2026-05-01");
    }
}
