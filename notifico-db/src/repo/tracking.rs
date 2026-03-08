use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use uuid::Uuid;

#[derive(Debug, Clone, FromQueryResult)]
pub struct TrackingEventRow {
    pub id: String,
    pub delivery_log_id: Option<String>,
    pub event_type: String,
    pub url: Option<String>,
    pub created_at: String,
}

pub async fn insert_tracking_event(
    db: &DatabaseConnection,
    id: Uuid,
    delivery_log_id: &str,
    event_type: &str,
    url: Option<&str>,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO tracking_event (id, delivery_log_id, event_type, url) VALUES (?, ?, ?, ?)",
        [
            id.to_string().into(),
            delivery_log_id.into(),
            event_type.into(),
            url.unwrap_or("").into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn count_by_delivery(
    db: &DatabaseConnection,
    delivery_log_id: &str,
) -> Result<Vec<(String, i64)>, DbErr> {
    #[derive(FromQueryResult)]
    struct TypeCount {
        event_type: String,
        cnt: i64,
    }

    let rows = TypeCount::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT event_type, COUNT(*) as cnt FROM tracking_event WHERE delivery_log_id = ? GROUP BY event_type",
        [delivery_log_id.into()],
    ))
    .all(db)
    .await?;

    Ok(rows.into_iter().map(|r| (r.event_type, r.cnt)).collect())
}
