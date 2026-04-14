use rusqlite::{Connection, Row};

use crate::models::{PlanTemplate};
use crate::StorageError;

/// Saves a plan template (INSERT for new, UPDATE for existing).
pub fn save_plan(conn: &Connection, plan: &PlanTemplate) -> Result<i64, StorageError> {
    let wordbooks_json = plan.to_json_wordbooks();
    let is_active_int: i64 = if plan.is_active { 1 } else { 0 };

    match plan.id {
        Some(id) => {
            conn.execute(
                "UPDATE plan_templates
                 SET name = ?1, active_wordbook_ids = ?2,
                     new_words_per_day = ?3, review_words_per_day = ?4,
                     mixed_test_per_day = ?5, wrong_word_test_per_day = ?6,
                     growth_interval_days = ?7, growth_increment = ?8,
                     is_active = ?9, updated_at = datetime('now')
                 WHERE id = ?10",
                rusqlite::params![
                    plan.name,
                    wordbooks_json,
                    plan.new_words_per_day,
                    plan.review_words_per_day,
                    plan.mixed_test_per_day,
                    plan.wrong_word_test_per_day,
                    plan.growth_interval_days,
                    plan.growth_increment,
                    is_active_int,
                    id,
                ],
            )
            .map_err(|e| StorageError::Database(format!("Failed to update plan {id}: {e}")))?;
            Ok(id)
        }
        None => {
            conn.execute(
                "INSERT INTO plan_templates
                     (name, active_wordbook_ids,
                      new_words_per_day, review_words_per_day,
                      mixed_test_per_day, wrong_word_test_per_day,
                      growth_interval_days, growth_increment, is_active)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    plan.name,
                    wordbooks_json,
                    plan.new_words_per_day,
                    plan.review_words_per_day,
                    plan.mixed_test_per_day,
                    plan.wrong_word_test_per_day,
                    plan.growth_interval_days,
                    plan.growth_increment,
                    is_active_int,
                ],
            )
            .map_err(|e| StorageError::Database(format!("Failed to insert plan: {e}")))?;
            Ok(conn.last_insert_rowid())
        }
    }
}

/// Returns the single active plan template, if one exists.
pub fn get_active_plan(conn: &Connection) -> Result<Option<PlanTemplate>, StorageError> {
    let result = conn.query_row(
        "SELECT id, name, active_wordbook_ids,
                new_words_per_day, review_words_per_day,
                mixed_test_per_day, wrong_word_test_per_day,
                growth_interval_days, growth_increment,
                is_active, created_at, updated_at
         FROM plan_templates
         WHERE is_active = 1
         LIMIT 1",
        [],
        |row| row_to_plan(row),
    );

    match result {
        Ok(plan) => Ok(Some(plan)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(StorageError::Database(format!("Failed to query plan: {e}"))),
    }
}

/// Returns all plan templates.
pub fn get_all_plans(conn: &Connection) -> Result<Vec<PlanTemplate>, StorageError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, active_wordbook_ids,
                    new_words_per_day, review_words_per_day,
                    mixed_test_per_day, wrong_word_test_per_day,
                    growth_interval_days, growth_increment,
                    is_active, created_at, updated_at
             FROM plan_templates
             ORDER BY created_at DESC",
        )
        .map_err(|e| StorageError::Database(format!("Failed to prepare plans query: {e}")))?;

    let rows = stmt
        .query_map([], |row| row_to_plan(row))
        .map_err(|e| StorageError::Database(format!("Failed to query plans: {e}")))?;

    let mut plans = Vec::new();
    for row in rows {
        plans.push(row.map_err(|e| StorageError::Database(format!("Failed to read plan: {e}")))?);
    }

    Ok(plans)
}

/// Sets a specific plan as active, deactivating all others.
pub fn set_active_plan(conn: &Connection, plan_id: i64) -> Result<(), StorageError> {
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_templates WHERE id = ?1",
            rusqlite::params![plan_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|count| count > 0)
        .map_err(|e| StorageError::Database(format!("Failed to check plan: {e}")))?;

    if !exists {
        return Err(StorageError::NotFound(format!("Plan {plan_id} not found")));
    }

    conn.execute("UPDATE plan_templates SET is_active = 0", [])
        .map_err(|e| StorageError::Database(format!("Failed to deactivate plans: {e}")))?;

    conn.execute(
        "UPDATE plan_templates SET is_active = 1, updated_at = datetime('now') WHERE id = ?1",
        rusqlite::params![plan_id],
    )
    .map_err(|e| StorageError::Database(format!("Failed to activate plan: {e}")))?;

    Ok(())
}

fn row_to_plan(row: &Row) -> Result<PlanTemplate, rusqlite::Error> {
    let is_active_raw: i64 = row.get(9)?;
    let wordbooks_json: String = row.get(2)?;

    Ok(PlanTemplate {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        active_wordbook_ids: PlanTemplate::from_json_wordbooks(&wordbooks_json),
        new_words_per_day: row.get(3)?,
        review_words_per_day: row.get(4)?,
        mixed_test_per_day: row.get(5)?,
        wrong_word_test_per_day: row.get(6)?,
        growth_interval_days: row.get(7)?,
        growth_increment: row.get(8)?,
        is_active: is_active_raw != 0,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}
