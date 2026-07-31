use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::DomainContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrongWordProjectionInput {
    pub entry_source_id: String,
    pub word: String,
    pub error_count: u64,
    #[serde(default)]
    pub correct_count: u64,
    #[serde(default)]
    pub consecutive_correct: u64,
    pub last_wrong_at: String,
    #[serde(default)]
    pub correct_since_last_wrong: u64,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WrongWordProjection {
    pub entry_source_id: String,
    pub word: String,
    pub error_count: u64,
    pub correct_count: u64,
    pub consecutive_correct: u64,
    pub last_wrong_at: String,
    pub correct_since_last_wrong: u64,
    pub error_rate: f64,
    pub priority_score: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportHistoryInput {
    pub date: String,
    pub mode: String,
    pub summary: ReportHistorySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportHistorySummary {
    pub total_questions: u64,
    pub correct_count: u64,
    pub total_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDay {
    pub date: String,
    pub total_questions: u64,
    pub correct_count: u64,
    pub accuracy_percent: f64,
    pub study_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportModeSummary {
    pub mode: String,
    pub total_questions: u64,
    pub correct_count: u64,
    pub accuracy_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreakInfo {
    pub current_streak: u64,
    pub longest_streak: u64,
    pub last_study_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportProjection {
    pub total_study_days: u64,
    pub total_words_learned: u64,
    pub total_questions_answered: u64,
    pub overall_accuracy: f64,
    pub streak_info: StreakInfo,
    pub mode_breakdown: Vec<ReportModeSummary>,
    pub last7_days: Vec<ReportDay>,
    pub daily_series: Vec<ReportDay>,
    pub mode_series: BTreeMap<String, Vec<ReportDay>>,
}

fn default_true() -> bool {
    true
}

pub fn project_wrong_words(
    context: &DomainContext,
    entries: &[WrongWordProjectionInput],
    filter: &str,
) -> Vec<WrongWordProjection> {
    let mut result = entries
        .iter()
        .filter(|entry| entry.error_count > 0 && !entry.entry_source_id.trim().is_empty())
        .map(|entry| score_wrong_word(context, entry))
        .collect::<Vec<_>>();

    result.sort_by(|left, right| {
        right
            .is_active
            .cmp(&left.is_active)
            .then_with(|| {
                if filter == "recent" {
                    right.last_wrong_at.cmp(&left.last_wrong_at)
                } else {
                    right
                        .priority_score
                        .partial_cmp(&left.priority_score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                }
            })
            .then_with(|| right.error_count.cmp(&left.error_count))
            .then_with(|| right.last_wrong_at.cmp(&left.last_wrong_at))
    });
    if filter == "highPriority" {
        result.retain(|entry| entry.priority_score >= 8.0);
    }
    result
}

pub fn score_wrong_word(
    context: &DomainContext,
    entry: &WrongWordProjectionInput,
) -> WrongWordProjection {
    let errors = entry.error_count as f64;
    let correct = entry.correct_count as f64;
    let error_rate = if errors + correct > 0.0 {
        errors / (errors + correct)
    } else {
        1.0
    };
    let recency = chrono::NaiveDate::parse_from_str(&context.local_day, "%Y-%m-%d")
        .ok()
        .map(|today| recency_score_at(&entry.last_wrong_at, today))
        .unwrap_or(0.0);
    let priority_score = round_score(
        ((errors.sqrt() * 1.8).min(4.5) + error_rate * 3.0 + (recency / 21.0) * 2.0 + 0.8
            - (entry.correct_since_last_wrong as f64 * 1.6).min(5.0)
            - (correct.sqrt() * 0.35).min(2.0))
        .clamp(0.0, 10.0),
    );
    WrongWordProjection {
        entry_source_id: entry.entry_source_id.clone(),
        word: entry.word.clone(),
        error_count: entry.error_count,
        correct_count: entry.correct_count,
        consecutive_correct: entry.consecutive_correct,
        last_wrong_at: entry.last_wrong_at.clone(),
        correct_since_last_wrong: entry.correct_since_last_wrong,
        error_rate: round_score(error_rate),
        priority_score,
        is_active: entry.is_active,
    }
}

pub fn project_report(
    context: &DomainContext,
    history: &[ReportHistoryInput],
    learned_count: u64,
) -> ReportProjection {
    let mut by_date = BTreeMap::<String, (u64, u64, u64)>::new();
    let mut by_mode = BTreeMap::<String, (u64, u64)>::new();
    let mut by_mode_date = BTreeMap::<String, BTreeMap<String, (u64, u64, u64)>>::new();

    for item in history
        .iter()
        .filter(|item| !item.date.is_empty() && item.summary.total_questions > 0)
    {
        let mode = normalize_mode(&item.mode);
        accumulate_day(by_date.entry(item.date.clone()).or_default(), &item.summary);
        let mode_total = by_mode.entry(mode.clone()).or_default();
        mode_total.0 += item.summary.total_questions;
        mode_total.1 += item.summary.correct_count;
        accumulate_day(
            by_mode_date
                .entry(mode)
                .or_default()
                .entry(item.date.clone())
                .or_default(),
            &item.summary,
        );
    }

    let daily_series = by_date
        .iter()
        .map(|(date, totals)| report_day(date, *totals))
        .collect::<Vec<_>>();
    let total_questions_answered = daily_series.iter().map(|day| day.total_questions).sum();
    let total_correct = daily_series
        .iter()
        .map(|day| day.correct_count)
        .sum::<u64>();
    let modes = [
        "newWord",
        "review",
        "mixedTest",
        "wrongWordReinforcement",
        "rootAffix",
    ];
    let mode_breakdown = modes
        .iter()
        .map(|mode| {
            let (total_questions, correct_count) = by_mode.get(*mode).copied().unwrap_or_default();
            ReportModeSummary {
                mode: (*mode).to_string(),
                total_questions,
                correct_count,
                accuracy_percent: accuracy(correct_count, total_questions),
            }
        })
        .collect();
    let mode_series = modes
        .iter()
        .map(|mode| {
            let series = by_mode_date
                .get(*mode)
                .map(|days| {
                    days.iter()
                        .map(|(date, totals)| report_day(date, *totals))
                        .collect()
                })
                .unwrap_or_default();
            ((*mode).to_string(), series)
        })
        .collect();
    let last7_days = recent_days(&by_date, &context.local_day, 7);
    let study_days = by_date.keys().cloned().collect::<Vec<_>>();

    ReportProjection {
        total_study_days: study_days.len() as u64,
        total_words_learned: learned_count,
        total_questions_answered,
        overall_accuracy: accuracy(total_correct, total_questions_answered),
        streak_info: streak_info(&study_days, &context.local_day),
        mode_breakdown,
        last7_days,
        daily_series,
        mode_series,
    }
}

fn normalize_mode(mode: &str) -> String {
    mode.strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(mode)
        .to_string()
}

fn accumulate_day(target: &mut (u64, u64, u64), summary: &ReportHistorySummary) {
    target.0 += summary.total_questions;
    target.1 += summary.correct_count;
    target.2 += summary.total_time_ms;
}

fn report_day(date: &str, totals: (u64, u64, u64)) -> ReportDay {
    ReportDay {
        date: date.to_string(),
        total_questions: totals.0,
        correct_count: totals.1,
        accuracy_percent: accuracy(totals.1, totals.0),
        study_time_ms: totals.2,
    }
}

fn recent_days(
    by_date: &BTreeMap<String, (u64, u64, u64)>,
    local_day: &str,
    days: usize,
) -> Vec<ReportDay> {
    let Ok(end) = chrono::NaiveDate::parse_from_str(local_day, "%Y-%m-%d") else {
        return Vec::new();
    };
    (0..days)
        .rev()
        .map(|offset| {
            let date = end - chrono::Days::new(offset as u64);
            let key = date.format("%Y-%m-%d").to_string();
            report_day(&key, by_date.get(&key).copied().unwrap_or_default())
        })
        .collect()
}

fn streak_info(study_days: &[String], local_day: &str) -> StreakInfo {
    let parsed = study_days
        .iter()
        .filter_map(|day| chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok())
        .collect::<Vec<_>>();
    let mut longest_streak = 0u64;
    let mut running = 0u64;
    let mut previous: Option<chrono::NaiveDate> = None;
    for day in &parsed {
        running = if previous.is_some_and(|prior| (*day - prior).num_days() == 1) {
            running + 1
        } else {
            1
        };
        longest_streak = longest_streak.max(running);
        previous = Some(*day);
    }
    let current_streak = chrono::NaiveDate::parse_from_str(local_day, "%Y-%m-%d")
        .ok()
        .map(|mut cursor| {
            let mut count = 0;
            while parsed.binary_search(&cursor).is_ok() {
                count += 1;
                cursor -= chrono::TimeDelta::days(1);
            }
            count
        })
        .unwrap_or(0);
    StreakInfo {
        current_streak,
        longest_streak,
        last_study_date: study_days.last().cloned(),
    }
}

fn recency_score_at(last_wrong_at: &str, today: chrono::NaiveDate) -> f64 {
    let date = last_wrong_at.get(0..10).unwrap_or(last_wrong_at);
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map(|wrong_day| (21 - (today - wrong_day).num_days().max(0).min(21)) as f64)
        .unwrap_or(0.0)
}

fn accuracy(correct: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (correct as f64 / total as f64) * 100.0
    }
}

fn round_score(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod phase4_projection_tests {
    use super::*;

    fn fixture() -> ProjectLearningEvidenceInput {
        serde_json::from_str(include_str!(
            "../../../fixtures/domain/v1/phase4-projections.json"
        ))
        .expect("valid Phase 4 projection fixture")
    }

    fn exam<'a>(result: &'a ProjectLearningEvidence, entry_source_id: &str) -> &'a ExamEvidence {
        result
            .exam
            .iter()
            .find(|row| row.identity.entry_source_id == entry_source_id)
            .expect("exam evidence")
    }

    #[test]
    fn phase4_projection_locks_policy_versions_severity_decay_and_recovery() {
        let result = project_learning_evidence(&fixture());

        assert_eq!(result.ordinary_policy_version, "mobile-risk-v1");
        assert_eq!(result.exam_policy_version, "exam-signal-v1");
        assert_eq!(result.practice_policy_version, "practice-priority-v1");

        let severities = ["severity-fuzzy", "severity-familiar", "severity-unknown", "severity-wrong"]
            .map(|id| exam(&result, id).exam_signal);
        assert!(severities.windows(2).all(|pair| pair[0] < pair[1]));

        let one = exam(&result, "repeat-one").exam_signal;
        let two = exam(&result, "repeat-two").exam_signal;
        assert!(two > one);
        assert!(two - one < one);
        assert!(exam(&result, "decay-old").exam_signal < exam(&result, "decay-fresh").exam_signal);

        assert_eq!(exam(&result, "recover-one").recovery_multiplier, 1.0);
        assert_eq!(exam(&result, "recover-two").recovery_multiplier, 0.7);
        assert_eq!(exam(&result, "recover-reset").recovery_multiplier, 1.0);
    }

    #[test]
    fn phase4_projection_separates_exam_marks_and_suppresses_duplicate_answers() {
        let result = project_learning_evidence(&fixture());
        let high = result
            .ordinary
            .iter()
            .find(|row| row.identity.entry_source_id == "ordinary-high")
            .expect("ordinary high-risk evidence");

        assert!(high.is_active);
        assert!(high.ordinary_risk_score >= 8.0);
        assert!(result.high_risk.iter().any(|identity| identity.entry_source_id == "ordinary-high"));
        assert!(result.practice_eligible.iter().all(|row| (0.0..=10.0).contains(&row.practice_priority)));
        assert_eq!(result.provenance.accepted_event_ids.iter().filter(|id| id.as_str() == "report-correct").count(), 1);
        assert_eq!(result.provenance.accepted_event_ids.len(), 11);
        assert_eq!(result.provenance.exam_mark_ids.len(), 13);
    }

    #[test]
    fn phase4_projection_uses_generated_and_accepted_denominators() {
        let result = project_learning_evidence(&fixture());
        let report_day = result.daily_reports.iter().find(|row| row.local_day == "2026-07-28").unwrap();
        assert_eq!(report_day.generated_questions, 3);
        assert_eq!(report_day.answered_questions, 2);
        assert_eq!(report_day.correct_questions, 1);
        assert_eq!(report_day.inaccurate_questions, 1);
        assert_eq!(report_day.skipped_questions, 1);
        assert_eq!(report_day.completion, Some(0.67));
        assert_eq!(report_day.accuracy, Some(0.5));

        let empty_day = result.daily_reports.iter().find(|row| row.local_day == "2026-07-31").unwrap();
        assert_eq!(empty_day.generated_questions, 1);
        assert_eq!(empty_day.answered_questions, 0);
        assert_eq!(empty_day.completion, Some(0.0));
        assert_eq!(empty_day.accuracy, None);

        let zero_denominator = result.daily_reports.iter().find(|row| row.local_day == "2026-07-30").unwrap();
        assert_eq!(zero_denominator.generated_questions, 0);
        assert_eq!(zero_denominator.completion, None);
    }
}
