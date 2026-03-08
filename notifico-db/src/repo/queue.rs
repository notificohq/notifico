use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_name: String,
    pub recipient_id: Uuid,
    pub channel: String,
    pub contact_value: String,
    pub rendered_body: Value,
    pub idempotency_key: Option<String>,
    pub rule_id: Option<Uuid>,
    pub status: String,
    pub attempt: i32,
    pub max_attempts: i32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, FromQueryResult)]
struct TaskRaw {
    id: String,
    project_id: String,
    event_name: String,
    recipient_id: String,
    channel: String,
    contact_value: String,
    rendered_body: String,
    idempotency_key: Option<String>,
    rule_id: Option<String>,
    status: String,
    attempt: i32,
    max_attempts: i32,
    error_message: Option<String>,
}

impl TaskRaw {
    fn into_row(self) -> Result<TaskRow, DbErr> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DbErr::Custom(format!("invalid id UUID: {e}")))?;
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| DbErr::Custom(format!("invalid project_id UUID: {e}")))?;
        let recipient_id = Uuid::parse_str(&self.recipient_id)
            .map_err(|e| DbErr::Custom(format!("invalid recipient_id UUID: {e}")))?;
        let rendered_body: Value = serde_json::from_str(&self.rendered_body)
            .map_err(|e| DbErr::Custom(format!("invalid rendered_body JSON: {e}")))?;
        let rule_id = self
            .rule_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|e| DbErr::Custom(format!("invalid rule_id UUID: {e}")))?;
        Ok(TaskRow {
            id,
            project_id,
            event_name: self.event_name,
            recipient_id,
            channel: self.channel,
            contact_value: self.contact_value,
            rendered_body,
            idempotency_key: self.idempotency_key,
            rule_id,
            status: self.status,
            attempt: self.attempt,
            max_attempts: self.max_attempts,
            error_message: self.error_message,
        })
    }
}

/// Insert a new delivery task with status='pending'.
pub async fn enqueue(
    db: &DatabaseConnection,
    id: Uuid,
    project_id: Uuid,
    event_name: &str,
    recipient_id: Uuid,
    channel: &str,
    contact_value: &str,
    rendered_body: &Value,
    idempotency_key: Option<&str>,
    max_attempts: i32,
    rule_id: Option<Uuid>,
) -> Result<(), DbErr> {
    let body_json = serde_json::to_string(rendered_body)
        .map_err(|e| DbErr::Custom(format!("JSON serialize error: {e}")))?;
    let idem = idempotency_key.unwrap_or("");
    let has_idem = idempotency_key.is_some();

    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO delivery_task (id, project_id, event_name, recipient_id, channel, contact_value, rendered_body, idempotency_key, rule_id, status, attempt, max_attempts) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'pending', 0, ?)",
        [
            id.to_string().into(),
            project_id.to_string().into(),
            event_name.into(),
            recipient_id.to_string().into(),
            channel.into(),
            contact_value.into(),
            body_json.into(),
            if has_idem { sea_orm::Value::from(idem) } else { sea_orm::Value::from(None::<String>) },
            rule_id.map(|r| r.to_string()).map(sea_orm::Value::from).unwrap_or(sea_orm::Value::from(None::<String>)),
            max_attempts.into(),
        ],
    ))
    .await?;

    Ok(())
}

/// Claim up to `limit` pending tasks (set status='processing', increment attempt).
/// Returns claimed tasks.
pub async fn claim_pending(
    db: &DatabaseConnection,
    limit: u32,
) -> Result<Vec<TaskRow>, DbErr> {
    // Step 1: Find pending task IDs ready to process
    let rows = TaskRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, project_id, event_name, recipient_id, channel, contact_value, rendered_body, idempotency_key, rule_id, status, attempt, max_attempts, error_message FROM delivery_task WHERE status = 'pending' AND next_retry_at <= CURRENT_TIMESTAMP ORDER BY next_retry_at ASC LIMIT ?",
        [limit.into()],
    ))
    .all(db)
    .await?;

    if rows.is_empty() {
        return Ok(vec![]);
    }

    // Step 2: Update status to 'processing' and increment attempt
    let ids: Vec<String> = rows.iter().map(|r| format!("'{}'", r.id)).collect();
    let id_list = ids.join(", ");
    db.execute_unprepared(&format!(
        "UPDATE delivery_task SET status = 'processing', attempt = attempt + 1, updated_at = CURRENT_TIMESTAMP WHERE id IN ({id_list})"
    ))
    .await?;

    // Return with incremented attempt
    let mut result = Vec::with_capacity(rows.len());
    for raw in rows {
        let mut row = raw.into_row()?;
        row.attempt += 1; // reflect the increment
        row.status = "processing".into();
        result.push(row);
    }
    Ok(result)
}

/// Mark task as completed.
pub async fn mark_completed(db: &DatabaseConnection, task_id: Uuid) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE delivery_task SET status = 'completed', updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        [task_id.to_string().into()],
    ))
    .await?;
    Ok(())
}

/// Mark task as failed. If retryable and attempts < max, requeue with backoff.
/// Otherwise mark as 'dead_letter'.
pub async fn mark_failed(
    db: &DatabaseConnection,
    task_id: Uuid,
    error: &str,
    retryable: bool,
    attempt: i32,
    max_attempts: i32,
) -> Result<(), DbErr> {
    if retryable && attempt < max_attempts {
        // Exponential backoff: 30s * 4^(attempt-1) → 30s, 2m, 8m, 32m
        let backoff_secs = 30i64 * 4i64.pow((attempt - 1).max(0) as u32);
        let sql = format!(
            "UPDATE delivery_task SET status = 'pending', error_message = ?, next_retry_at = datetime('now', '+{backoff_secs} seconds'), updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        );
        db.execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            &sql,
            [error.into(), task_id.to_string().into()],
        ))
        .await?;
    } else {
        db.execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE delivery_task SET status = 'dead_letter', error_message = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            [error.into(), task_id.to_string().into()],
        ))
        .await?;
    }
    Ok(())
}

/// Count tasks by status.
pub async fn count_by_status(
    db: &DatabaseConnection,
) -> Result<Vec<(String, i64)>, DbErr> {
    #[derive(Debug, FromQueryResult)]
    struct StatusCount {
        status: String,
        count: i64,
    }

    let rows = StatusCount::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT status, COUNT(*) as count FROM delivery_task GROUP BY status",
        [],
    ))
    .all(db)
    .await?;

    Ok(rows.into_iter().map(|r| (r.status, r.count)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};
    use sea_orm::ConnectionTrait;
    use serde_json::json;

    async fn setup() -> DatabaseConnection {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        // Seed project and recipient for FK constraints
        let project_id = "00000000-0000-0000-0000-000000000001";
        let recipient_id = "00000000-0000-0000-0000-000000000002";
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();
        db.execute_unprepared(&format!(
            "INSERT INTO recipient (id, project_id, external_id) VALUES ('{recipient_id}', '{project_id}', 'ext-1')"
        ))
        .await
        .unwrap();
        db
    }

    fn test_project_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    fn test_recipient_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap()
    }

    #[tokio::test]
    async fn enqueue_and_claim() {
        let db = setup().await;
        let task_id = Uuid::now_v7();

        enqueue(
            &db, task_id, test_project_id(), "order.confirmed",
            test_recipient_id(), "email", "test@example.com",
            &json!({"subject": "Hi", "text": "Hello"}),
            None, 5, None,
        )
        .await
        .unwrap();

        let claimed = claim_pending(&db, 10).await.unwrap();
        assert_eq!(claimed.len(), 1);
        assert_eq!(claimed[0].id, task_id);
        assert_eq!(claimed[0].status, "processing");
        assert_eq!(claimed[0].attempt, 1);
        assert_eq!(claimed[0].channel, "email");
        assert_eq!(claimed[0].rendered_body["subject"], "Hi");
    }

    #[tokio::test]
    async fn claim_skips_future_retry() {
        let db = setup().await;
        let task_id = Uuid::now_v7();

        enqueue(
            &db, task_id, test_project_id(), "test",
            test_recipient_id(), "email", "a@b.com",
            &json!({}), None, 5, None,
        )
        .await
        .unwrap();

        // Set next_retry_at far in the future
        db.execute_unprepared(&format!(
            "UPDATE delivery_task SET next_retry_at = datetime('now', '+1 hour') WHERE id = '{task_id}'"
        ))
        .await
        .unwrap();

        let claimed = claim_pending(&db, 10).await.unwrap();
        assert!(claimed.is_empty());
    }

    #[tokio::test]
    async fn mark_completed_changes_status() {
        let db = setup().await;
        let task_id = Uuid::now_v7();

        enqueue(
            &db, task_id, test_project_id(), "test",
            test_recipient_id(), "email", "a@b.com",
            &json!({}), None, 5, None,
        )
        .await
        .unwrap();

        let claimed = claim_pending(&db, 10).await.unwrap();
        assert_eq!(claimed.len(), 1);

        mark_completed(&db, task_id).await.unwrap();

        // Should not be claimable again
        let claimed_again = claim_pending(&db, 10).await.unwrap();
        assert!(claimed_again.is_empty());

        // Verify status via count
        let counts = count_by_status(&db).await.unwrap();
        assert!(counts.iter().any(|(s, c)| s == "completed" && *c == 1));
    }

    #[tokio::test]
    async fn mark_failed_retryable_requeues() {
        let db = setup().await;
        let task_id = Uuid::now_v7();

        enqueue(
            &db, task_id, test_project_id(), "test",
            test_recipient_id(), "email", "a@b.com",
            &json!({}), None, 5, None,
        )
        .await
        .unwrap();

        let claimed = claim_pending(&db, 10).await.unwrap();
        assert_eq!(claimed.len(), 1);

        // Fail with retryable — attempt=1, max=5 → should go back to pending
        mark_failed(&db, task_id, "timeout", true, 1, 5).await.unwrap();

        let counts = count_by_status(&db).await.unwrap();
        assert!(counts.iter().any(|(s, c)| s == "pending" && *c == 1));
    }

    #[tokio::test]
    async fn mark_failed_exhausted_goes_dead_letter() {
        let db = setup().await;
        let task_id = Uuid::now_v7();

        enqueue(
            &db, task_id, test_project_id(), "test",
            test_recipient_id(), "email", "a@b.com",
            &json!({}), None, 2, None,
        )
        .await
        .unwrap();

        claim_pending(&db, 10).await.unwrap();

        // Fail with attempt >= max_attempts → dead_letter
        mark_failed(&db, task_id, "permanent error", true, 2, 2).await.unwrap();

        let counts = count_by_status(&db).await.unwrap();
        assert!(counts.iter().any(|(s, c)| s == "dead_letter" && *c == 1));
    }
}
