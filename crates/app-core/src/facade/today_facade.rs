//! Today home facade for daily study dashboard.

use rusqlite::Connection;

use word_storage_core::models::{
    DailyProgress, DailySnapshot, PlanSummary, TodayHomeState, TodayHomeStateSeed, WordbookSummary,
};
use word_storage_core::persistence;
use word_storage_core::StorageError;

/// Get today's home state.
///
/// Returns the full state needed for the today home dashboard,
/// including the active plan, today's snapshot, wordbooks, and progress.
///
/// # Example
///
/// ```rust,ignore
/// let state = get_today_home_state(&conn)?;
/// println!("Today: {}", state.today_date);
/// ```
pub fn get_today_home_state(conn: &Connection) -> Result<TodayHomeState, StorageError> {
    let today_date = chrono::Local::now().format("%Y-%m-%d").to_string();

    // Get active plan
    let active_plan = persistence::plan_repo::get_active_plan(conn)?.map(|plan| PlanSummary {
        id: plan.id.unwrap_or(0),
        name: plan.name,
        new_words_per_day: plan.new_words_per_day,
        review_words_per_day: plan.review_words_per_day,
        mixed_test_per_day: plan.mixed_test_per_day,
        wrong_word_test_per_day: plan.wrong_word_test_per_day,
        high_frequency_per_day: 0,
        root_affix_per_day: None,
        growth_interval_days: plan.growth_interval_days,
        growth_increment: plan.growth_increment,
        question_type_weights_by_mode: None,
    });

    // Get wordbooks
    let wordbooks = persistence::wordbook_repo::get_active_wordbooks(conn)?
        .into_iter()
        .map(|wb| WordbookSummary {
            id: wb.id.unwrap_or(0),
            code: wb.code,
            name: wb.name,
            category: wb.category,
            total_entries: wb.total_entries,
            is_active: wb.is_active,
        })
        .collect();

    // Build snapshot (placeholder - would come from snapshot service)
    let today_snapshot = active_plan.as_ref().map(|plan| DailySnapshot {
        date: today_date.clone(),
        new_words_target: plan.new_words_per_day as u32,
        new_words_base_target: Some(plan.new_words_per_day as u32),
        new_words_carryover_target: Some(0),
        new_words_completed: 0,
        review_words_target: plan.review_words_per_day as u32,
        review_words_base_target: Some(plan.review_words_per_day as u32),
        review_words_carryover_target: Some(0),
        review_words_completed: 0,
        mixed_test_target: plan.mixed_test_per_day as u32,
        mixed_test_base_target: Some(plan.mixed_test_per_day as u32),
        mixed_test_carryover_target: Some(0),
        mixed_test_completed: 0,
        wrong_word_test_target: plan.wrong_word_test_per_day as u32,
        wrong_word_test_base_target: Some(plan.wrong_word_test_per_day as u32),
        wrong_word_test_carryover_target: Some(0),
        wrong_word_test_completed: 0,
        high_frequency_target: plan.high_frequency_per_day as u32,
        high_frequency_base_target: Some(plan.high_frequency_per_day as u32),
        high_frequency_carryover_target: Some(0),
        high_frequency_completed: 0,
        root_affix_target: None,
        root_affix_base_target: Some(0),
        root_affix_carryover_target: Some(0),
        root_affix_completed: None,
    });

    // Build progress
    let daily_progress = DailyProgress {
        total_tasks: 4,
        completed_tasks: 0,
        next_recommended_action: "Start new words".to_string(),
    };

    Ok(TodayHomeState {
        today_date,
        active_plan,
        today_snapshot,
        wordbooks,
        daily_progress,
    })
}

pub fn build_today_home_state(seed: TodayHomeStateSeed) -> TodayHomeState {
    let targets = seed.targets;
    let today_snapshot = seed.active_plan.as_ref().map(|plan| DailySnapshot {
        date: seed.today_date.clone(),
        new_words_target: targets
            .new_words_target
            .unwrap_or(plan.new_words_per_day.max(0) as u32),
        new_words_base_target: targets.new_words_base_target,
        new_words_carryover_target: targets.new_words_carryover_target,
        new_words_completed: seed.completions.new_words_completed,
        review_words_target: targets
            .review_words_target
            .unwrap_or(plan.review_words_per_day.max(0) as u32),
        review_words_base_target: targets.review_words_base_target,
        review_words_carryover_target: targets.review_words_carryover_target,
        review_words_completed: seed.completions.review_words_completed,
        mixed_test_target: targets
            .mixed_test_target
            .unwrap_or(plan.mixed_test_per_day.max(0) as u32),
        mixed_test_base_target: targets.mixed_test_base_target,
        mixed_test_carryover_target: targets.mixed_test_carryover_target,
        mixed_test_completed: seed.completions.mixed_test_completed,
        wrong_word_test_target: targets
            .wrong_word_test_target
            .unwrap_or(plan.wrong_word_test_per_day.max(0) as u32),
        wrong_word_test_base_target: targets.wrong_word_test_base_target,
        wrong_word_test_carryover_target: targets.wrong_word_test_carryover_target,
        wrong_word_test_completed: seed.completions.wrong_word_test_completed,
        high_frequency_target: targets
            .high_frequency_target
            .unwrap_or(plan.high_frequency_per_day.max(0) as u32),
        high_frequency_base_target: targets.high_frequency_base_target,
        high_frequency_carryover_target: targets.high_frequency_carryover_target,
        high_frequency_completed: seed.completions.high_frequency_completed,
        root_affix_target: Some(targets.root_affix_target.unwrap_or_else(|| {
            plan.root_affix_per_day
                .map(|value| value.max(0) as u32)
                .unwrap_or(0)
        })),
        root_affix_base_target: targets.root_affix_base_target,
        root_affix_carryover_target: targets.root_affix_carryover_target,
        root_affix_completed: seed.completions.root_affix_completed,
    });

    let daily_progress =
        today_snapshot
            .as_ref()
            .map(build_daily_progress)
            .unwrap_or(DailyProgress {
                total_tasks: 0,
                completed_tasks: 0,
                next_recommended_action: "Start new words".to_string(),
            });

    TodayHomeState {
        today_date: seed.today_date,
        active_plan: seed.active_plan,
        today_snapshot,
        wordbooks: seed.wordbooks,
        daily_progress,
    }
}

fn build_daily_progress(snapshot: &DailySnapshot) -> DailyProgress {
    let total_tasks = snapshot.new_words_target
        + snapshot.review_words_target
        + snapshot.mixed_test_target
        + snapshot.wrong_word_test_target
        + snapshot.high_frequency_target
        + snapshot.root_affix_target.unwrap_or(0);
    let completed_tasks = snapshot.new_words_completed
        + snapshot.review_words_completed
        + snapshot.mixed_test_completed
        + snapshot.wrong_word_test_completed
        + snapshot.high_frequency_completed
        + snapshot.root_affix_completed.unwrap_or(0);

    let next_recommended_action = if snapshot.new_words_completed < snapshot.new_words_target {
        "Start new words".to_string()
    } else if snapshot.review_words_completed < snapshot.review_words_target {
        "Continue with review".to_string()
    } else if snapshot.mixed_test_completed < snapshot.mixed_test_target {
        "Continue with mixed test".to_string()
    } else if snapshot.wrong_word_test_completed < snapshot.wrong_word_test_target {
        "Continue with wrong-word reinforcement".to_string()
    } else if snapshot.high_frequency_completed < snapshot.high_frequency_target {
        "Continue with high-frequency words".to_string()
    } else if snapshot.root_affix_completed.unwrap_or(0) < snapshot.root_affix_target.unwrap_or(0) {
        "Continue with root/affix".to_string()
    } else {
        "Return to Today".to_string()
    };

    DailyProgress {
        total_tasks,
        completed_tasks,
        next_recommended_action,
    }
}
