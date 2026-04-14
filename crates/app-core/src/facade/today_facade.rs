//! Today home facade for daily study dashboard.

use rusqlite::Connection;

use word_storage_core::models::{TodayHomeState, PlanSummary, DailySnapshot, WordbookSummary, DailyProgress};
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
    let active_plan = persistence::plan_repo::get_active_plan(conn)?
        .map(|plan| PlanSummary {
            id: plan.id.unwrap_or(0),
            name: plan.name,
            new_words_per_day: plan.new_words_per_day,
            review_words_per_day: plan.review_words_per_day,
            mixed_test_per_day: plan.mixed_test_per_day,
            wrong_word_test_per_day: plan.wrong_word_test_per_day,
            growth_interval_days: plan.growth_interval_days,
            growth_increment: plan.growth_increment,
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
        new_words_completed: 0,
        review_words_target: plan.review_words_per_day as u32,
        review_words_completed: 0,
        mixed_test_target: plan.mixed_test_per_day as u32,
        mixed_test_completed: 0,
        wrong_word_test_target: plan.wrong_word_test_per_day as u32,
        wrong_word_test_completed: 0,
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
