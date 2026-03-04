use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, Statement};
use uuid::Uuid;

/// Build a compound idempotency key: event_name + recipient_id + channel + optional client key.
pub fn make_idempotency_key(
    event_name: &str,
    recipient_id: Uuid,
    channel: &str,
    client_key: Option<&str>,
) -> String {
    match client_key {
        Some(k) => format!("{event_name}:{recipient_id}:{channel}:{k}"),
        None => format!("{event_name}:{recipient_id}:{channel}"),
    }
}

/// Check if an idempotency key already exists. If not, insert it and return `false`.
/// If it already exists, return `true` (duplicate).
pub async fn check_and_insert(
    db: &DatabaseConnection,
    idempotency_key: &str,
) -> Result<bool, DbErr> {
    let exists = db
        .query_one_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT id FROM idempotency_record WHERE idempotency_key = ?",
            [idempotency_key.into()],
        ))
        .await?;

    if exists.is_some() {
        return Ok(true);
    }

    let id = Uuid::now_v7();
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO idempotency_record (id, idempotency_key) VALUES (?, ?)",
        [id.to_string().into(), idempotency_key.into()],
    ))
    .await?;

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};

    #[tokio::test]
    async fn idempotency_key_format() {
        let rid = Uuid::now_v7();
        let key = make_idempotency_key("order.confirmed", rid, "email", Some("abc"));
        assert!(key.contains("order.confirmed"));
        assert!(key.contains("email"));
        assert!(key.contains("abc"));

        let key_no_client = make_idempotency_key("order.confirmed", rid, "email", None);
        assert!(!key_no_client.contains("abc"));
        assert_eq!(
            key_no_client,
            format!("order.confirmed:{rid}:email")
        );
    }

    #[tokio::test]
    async fn check_and_insert_first_time() {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        let is_dup = check_and_insert(&db, "test-key-1").await.unwrap();
        assert!(!is_dup);
    }

    #[tokio::test]
    async fn check_and_insert_duplicate() {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        let first = check_and_insert(&db, "test-key-2").await.unwrap();
        assert!(!first);

        let second = check_and_insert(&db, "test-key-2").await.unwrap();
        assert!(second);
    }
}
