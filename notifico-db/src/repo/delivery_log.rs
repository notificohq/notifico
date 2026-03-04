use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DeliveryLogRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_name: String,
    pub recipient_id: Uuid,
    pub channel: String,
    pub status: String,
    pub error_message: Option<String>,
    pub attempts: i32,
    pub created_at: String,
    pub delivered_at: Option<String>,
}

#[derive(Debug, Clone, FromQueryResult)]
struct DeliveryLogRaw {
    id: String,
    project_id: String,
    event_name: String,
    recipient_id: String,
    channel: String,
    status: String,
    error_message: Option<String>,
    attempts: i32,
    created_at: String,
    delivered_at: Option<String>,
}

impl DeliveryLogRaw {
    fn into_row(self) -> Result<DeliveryLogRow, DbErr> {
        Ok(DeliveryLogRow {
            id: Uuid::parse_str(&self.id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            project_id: Uuid::parse_str(&self.project_id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            event_name: self.event_name,
            recipient_id: Uuid::parse_str(&self.recipient_id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            channel: self.channel,
            status: self.status,
            error_message: self.error_message,
            attempts: self.attempts,
            created_at: self.created_at,
            delivered_at: self.delivered_at,
        })
    }
}

/// Insert a delivery log entry.
pub async fn insert_log(
    db: &DatabaseConnection,
    id: Uuid,
    project_id: Uuid,
    event_name: &str,
    recipient_id: Uuid,
    channel: &str,
    status: &str,
    error_message: Option<&str>,
    attempts: i32,
) -> Result<(), DbErr> {
    let delivered_at = if status == "delivered" {
        "CURRENT_TIMESTAMP"
    } else {
        "NULL"
    };

    // Use raw SQL because we need CURRENT_TIMESTAMP expression
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        &format!(
            "INSERT INTO delivery_log (id, project_id, event_name, recipient_id, channel, status, error_message, attempts, delivered_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, {delivered_at})"
        ),
        [
            id.to_string().into(),
            project_id.to_string().into(),
            event_name.into(),
            recipient_id.to_string().into(),
            channel.into(),
            status.into(),
            error_message.map(|s| s.to_string()).into(),
            attempts.into(),
        ],
    ))
    .await?;
    Ok(())
}

/// List delivery logs for a project, newest first. Supports optional filters.
pub async fn list_logs(
    db: &DatabaseConnection,
    project_id: Uuid,
    status: Option<&str>,
    event_name: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<Vec<DeliveryLogRow>, DbErr> {
    let mut sql = String::from(
        "SELECT id, project_id, event_name, recipient_id, channel, status, error_message, attempts, created_at, delivered_at \
         FROM delivery_log WHERE project_id = ?"
    );
    let mut params: Vec<sea_orm::Value> = vec![project_id.to_string().into()];

    if let Some(s) = status {
        sql.push_str(" AND status = ?");
        params.push(s.into());
    }
    if let Some(e) = event_name {
        sql.push_str(" AND event_name = ?");
        params.push(e.into());
    }

    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {limit} OFFSET {offset}"));

    let rows = DeliveryLogRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        params,
    ))
    .all(db)
    .await?;
    rows.into_iter().map(|r| r.into_row()).collect()
}

/// Count delivery logs for a project with optional filters.
pub async fn count_logs(
    db: &DatabaseConnection,
    project_id: Uuid,
    status: Option<&str>,
    event_name: Option<&str>,
) -> Result<i64, DbErr> {
    #[derive(FromQueryResult)]
    struct CountResult {
        cnt: i64,
    }

    let mut sql =
        String::from("SELECT COUNT(*) as cnt FROM delivery_log WHERE project_id = ?");
    let mut params: Vec<sea_orm::Value> = vec![project_id.to_string().into()];

    if let Some(s) = status {
        sql.push_str(" AND status = ?");
        params.push(s.into());
    }
    if let Some(e) = event_name {
        sql.push_str(" AND event_name = ?");
        params.push(e.into());
    }

    let result = CountResult::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        params,
    ))
    .one(db)
    .await?
    .unwrap_or(CountResult { cnt: 0 });

    Ok(result.cnt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};
    use sea_orm::ConnectionTrait;

    async fn setup() -> (DatabaseConnection, Uuid, Uuid) {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        let project_id = Uuid::now_v7();
        let recipient_id = Uuid::now_v7();
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();
        db.execute_unprepared(&format!(
            "INSERT INTO recipient (id, project_id, external_id) VALUES ('{recipient_id}', '{project_id}', 'user-1')"
        ))
        .await
        .unwrap();

        (db, project_id, recipient_id)
    }

    #[tokio::test]
    async fn insert_and_list_logs() {
        let (db, project_id, recipient_id) = setup().await;

        insert_log(
            &db,
            Uuid::now_v7(),
            project_id,
            "order.confirmed",
            recipient_id,
            "email",
            "delivered",
            None,
            1,
        )
        .await
        .unwrap();

        insert_log(
            &db,
            Uuid::now_v7(),
            project_id,
            "order.confirmed",
            recipient_id,
            "sms",
            "failed",
            Some("SMTP timeout"),
            3,
        )
        .await
        .unwrap();

        let all = list_logs(&db, project_id, None, None, 50, 0).await.unwrap();
        assert_eq!(all.len(), 2);

        let delivered = list_logs(&db, project_id, Some("delivered"), None, 50, 0)
            .await
            .unwrap();
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].channel, "email");

        let count = count_logs(&db, project_id, None, None).await.unwrap();
        assert_eq!(count, 2);

        let count_failed = count_logs(&db, project_id, Some("failed"), None).await.unwrap();
        assert_eq!(count_failed, 1);
    }

    #[tokio::test]
    async fn list_logs_with_pagination() {
        let (db, project_id, recipient_id) = setup().await;

        for i in 0..5 {
            insert_log(
                &db,
                Uuid::now_v7(),
                project_id,
                &format!("event.{i}"),
                recipient_id,
                "email",
                "delivered",
                None,
                1,
            )
            .await
            .unwrap();
        }

        let page1 = list_logs(&db, project_id, None, None, 2, 0).await.unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = list_logs(&db, project_id, None, None, 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);

        let page3 = list_logs(&db, project_id, None, None, 2, 4).await.unwrap();
        assert_eq!(page3.len(), 1);
    }
}
