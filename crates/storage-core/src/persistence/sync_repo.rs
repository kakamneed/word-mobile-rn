use rusqlite::{params, Connection};

use crate::models::{SyncDomainPendingCount, SyncStatus};
use crate::StorageError;

pub fn enqueue_latest_outbox_item(
    conn: &Connection,
    domain: &str,
    payload_json: &str,
    idempotency_key: &str,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO sync_outbox (
            domain,
            payload_json,
            idempotency_key,
            status,
            created_at,
            attempt_count,
            last_attempt_at
        )
        VALUES (?1, ?2, ?3, 'pending', datetime('now'), 0, NULL)
        ON CONFLICT(idempotency_key) DO UPDATE SET
            domain = excluded.domain,
            payload_json = excluded.payload_json,
            status = 'pending',
            created_at = datetime('now'),
            last_attempt_at = NULL",
        params![domain, payload_json, idempotency_key],
    )
    .map_err(|e| StorageError::Database(format!("Failed to enqueue sync outbox item: {e}")))?;

    Ok(())
}

pub fn get_sync_status(conn: &Connection) -> Result<SyncStatus, StorageError> {
    let pending_count: i64 = conn
        .query_row(
            "SELECT COUNT(*)
             FROM sync_outbox
             WHERE status IN ('pending', 'in_flight', 'retryable_failure')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| StorageError::Database(format!("Failed to count sync outbox: {e}")))?;

    let last_sync_succeeded_at: Option<String> = conn
        .query_row(
            "SELECT MAX(last_attempt_at)
             FROM sync_outbox
             WHERE status = 'succeeded'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| {
            StorageError::Database(format!(
                "Failed to read last successful sync timestamp: {e}"
            ))
        })?;

    let last_dead_letter_error = conn
        .query_row(
            "SELECT failure_code
             FROM sync_dead_letter
             ORDER BY datetime(created_at) DESC, id DESC
             LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();

    let retryable_error = conn
        .query_row(
            "SELECT status
             FROM sync_outbox
             WHERE status = 'retryable_failure'
             ORDER BY datetime(last_attempt_at) DESC, id DESC
             LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();

    let mut stmt = conn
        .prepare(
            "SELECT domain, COUNT(*)
             FROM sync_outbox
             WHERE status IN ('pending', 'in_flight', 'retryable_failure')
             GROUP BY domain
             ORDER BY domain ASC",
        )
        .map_err(|e| {
            StorageError::Database(format!("Failed to prepare sync domain pending query: {e}"))
        })?;

    let domains_pending = stmt
        .query_map([], |row| {
            Ok(SyncDomainPendingCount {
                domain: row.get(0)?,
                pending_count: row.get(1)?,
            })
        })
        .map_err(|e| {
            StorageError::Database(format!("Failed to query sync domain pending counts: {e}"))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| {
            StorageError::Database(format!("Failed to decode sync domain pending counts: {e}"))
        })?;

    Ok(SyncStatus {
        sync_enabled: false,
        transport_configured: false,
        account_sync_state: if pending_count > 0 {
            "local_queue_pending".to_string()
        } else {
            "local_queue_idle".to_string()
        },
        pending_count,
        last_sync_succeeded_at,
        last_sync_error_code: last_dead_letter_error.or(retryable_error),
        domains_pending,
    })
}

pub fn record_dead_letter(
    conn: &Connection,
    domain: &str,
    payload_json: &str,
    idempotency_key: &str,
    failure_code: &str,
    failure_message: &str,
) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO sync_dead_letter (
            domain,
            payload_json,
            idempotency_key,
            failure_code,
            failure_message,
            created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
        params![
            domain,
            payload_json,
            idempotency_key,
            failure_code,
            failure_message
        ],
    )
    .map_err(|e| StorageError::Database(format!("Failed to record sync dead letter: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::schema;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory database");
        schema::apply_schema(&conn).expect("apply schema");
        conn
    }

    #[test]
    fn enqueued_plan_config_appears_in_read_only_status() {
        let conn = test_conn();

        enqueue_latest_outbox_item(
            &conn,
            "plan_config",
            r#"{"name":"Test plan"}"#,
            "plan_config:saved_plan_json",
        )
        .expect("enqueue outbox item");

        let status = get_sync_status(&conn).expect("read sync status");

        assert!(!status.sync_enabled);
        assert!(!status.transport_configured);
        assert_eq!(status.account_sync_state, "local_queue_pending");
        assert_eq!(status.pending_count, 1);
        assert_eq!(status.domains_pending.len(), 1);
        assert_eq!(status.domains_pending[0].domain, "plan_config");
        assert_eq!(status.domains_pending[0].pending_count, 1);
    }

    #[test]
    fn enqueue_latest_reuses_idempotency_key_for_config_snapshot() {
        let conn = test_conn();

        enqueue_latest_outbox_item(
            &conn,
            "plan_config",
            r#"{"version":1}"#,
            "plan_config:saved_plan_json",
        )
        .expect("enqueue first item");
        enqueue_latest_outbox_item(
            &conn,
            "plan_config",
            r#"{"version":2}"#,
            "plan_config:saved_plan_json",
        )
        .expect("enqueue replacement item");

        let status = get_sync_status(&conn).expect("read sync status");
        let payload: String = conn
            .query_row("SELECT payload_json FROM sync_outbox", [], |row| row.get(0))
            .expect("read payload");

        assert_eq!(status.pending_count, 1);
        assert_eq!(payload, r#"{"version":2}"#);
    }

    #[test]
    fn dead_letter_error_is_visible_in_status_without_pending_queue() {
        let conn = test_conn();

        record_dead_letter(
            &conn,
            "plan_config",
            r#"{"version":2}"#,
            "plan_config:saved_plan_json",
            "enqueue_failed",
            "synthetic failure",
        )
        .expect("record dead letter");

        let status = get_sync_status(&conn).expect("read sync status");

        assert_eq!(status.pending_count, 0);
        assert_eq!(
            status.last_sync_error_code.as_deref(),
            Some("enqueue_failed")
        );
        assert_eq!(status.account_sync_state, "local_queue_idle");
    }
}
