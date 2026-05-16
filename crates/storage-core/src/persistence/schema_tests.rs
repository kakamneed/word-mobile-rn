use rusqlite::Connection;

use super::schema;

#[test]
fn reward_image_and_local_leaderboard_tables_are_created_without_data_loss() {
    let conn = Connection::open_in_memory().expect("open in-memory database");
    schema::apply_schema(&conn).expect("apply initial schema");

    conn.execute(
        "INSERT INTO reward_image_upload_entitlements (
            owner_key,
            available_uploads,
            last_granted_streak_milestone
        ) VALUES ('local', 1, 1)",
        [],
    )
    .expect("insert entitlement");
    conn.execute(
        "INSERT INTO reward_images (
            owner_key,
            local_path,
            mime_type,
            original_filename,
            moderation_status
        ) VALUES ('local', '/tmp/reward.webp', 'image/webp', 'reward.webp', 'approved')",
        [],
    )
    .expect("insert reward image");
    let image_id = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO reward_image_votes (image_id, voter_key, week_start)
         VALUES (?1, 'local', '2026-05-04')",
        [image_id],
    )
    .expect("insert reward image vote");
    conn.execute(
        "INSERT INTO local_leaderboard_summaries (
            user_key,
            display_name,
            total_questions,
            correct_count,
            period,
            period_start
        ) VALUES ('local', 'Local learner', 12, 10, 'weekly', '2026-05-04')",
        [],
    )
    .expect("insert local leaderboard summary");

    schema::apply_schema(&conn).expect("reapply schema");

    assert_eq!(row_count(&conn, "reward_image_upload_entitlements"), 1);
    assert_eq!(row_count(&conn, "reward_images"), 1);
    assert_eq!(row_count(&conn, "reward_image_votes"), 1);
    assert_eq!(row_count(&conn, "local_leaderboard_summaries"), 1);
}

fn row_count(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .expect("count rows")
}
