use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct MiddlewareRow {
    pub id: Uuid,
    pub rule_id: Uuid,
    pub middleware_name: String,
    pub config: String,
    pub priority: i32,
    pub enabled: bool,
}

impl MiddlewareRow {
    /// Parse the `config` string into a `serde_json::Value`.
    pub fn config_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::from_str(&self.config)
    }
}

#[derive(Debug, Clone, FromQueryResult)]
struct MiddlewareRaw {
    id: String,
    rule_id: String,
    middleware_name: String,
    config: String,
    priority: i32,
    enabled: i32,
}

impl MiddlewareRaw {
    fn into_row(self) -> Result<MiddlewareRow, DbErr> {
        Ok(MiddlewareRow {
            id: Uuid::parse_str(&self.id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            rule_id: Uuid::parse_str(&self.rule_id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            middleware_name: self.middleware_name,
            config: self.config,
            priority: self.priority,
            enabled: self.enabled != 0,
        })
    }
}

/// List enabled middleware for a rule, ordered by priority ASC.
pub async fn list_by_rule(
    db: &DatabaseConnection,
    rule_id: Uuid,
) -> Result<Vec<MiddlewareRow>, DbErr> {
    let rows = MiddlewareRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, rule_id, middleware_name, config, priority, enabled \
         FROM pipeline_middleware \
         WHERE rule_id = ? AND enabled = 1 \
         ORDER BY priority ASC",
        [rule_id.to_string().into()],
    ))
    .all(db)
    .await?;
    rows.into_iter().map(|r| r.into_row()).collect()
}

/// List all middleware for a rule (including disabled), ordered by priority ASC.
pub async fn list_all_by_rule(
    db: &DatabaseConnection,
    rule_id: Uuid,
) -> Result<Vec<MiddlewareRow>, DbErr> {
    let rows = MiddlewareRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, rule_id, middleware_name, config, priority, enabled \
         FROM pipeline_middleware \
         WHERE rule_id = ? \
         ORDER BY priority ASC",
        [rule_id.to_string().into()],
    ))
    .all(db)
    .await?;
    rows.into_iter().map(|r| r.into_row()).collect()
}

/// Insert a new middleware entry.
pub async fn insert(
    db: &DatabaseConnection,
    id: Uuid,
    rule_id: Uuid,
    middleware_name: &str,
    config: &Value,
    priority: i32,
) -> Result<(), DbErr> {
    db.execute_raw(
        Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO pipeline_middleware (id, rule_id, middleware_name, config, priority) \
             VALUES (?, ?, ?, ?, ?)",
            [
                id.to_string().into(),
                rule_id.to_string().into(),
                middleware_name.into(),
                config.to_string().into(),
                priority.into(),
            ],
        ),
    )
    .await?;
    Ok(())
}

/// Update an existing middleware entry.
pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    config: &Value,
    priority: i32,
    enabled: bool,
) -> Result<(), DbErr> {
    db.execute_raw(
        Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE pipeline_middleware SET config = ?, priority = ?, enabled = ? WHERE id = ?",
            [
                config.to_string().into(),
                priority.into(),
                (enabled as i32).into(),
                id.to_string().into(),
            ],
        ),
    )
    .await?;
    Ok(())
}

/// Delete a middleware entry by id.
pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    db.execute_raw(
        Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM pipeline_middleware WHERE id = ?",
            [id.to_string().into()],
        ),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};
    use sea_orm::ConnectionTrait;

    /// Create an in-memory SQLite DB, run migrations, and seed the FK chain
    /// (project -> event, template, pipeline_rule) needed for pipeline_middleware.
    async fn setup() -> (DatabaseConnection, Uuid) {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        let project_id = Uuid::now_v7();
        let event_id = Uuid::now_v7();
        let template_id = Uuid::now_v7();
        let rule_id = Uuid::now_v7();

        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO event (id, project_id, name, category) \
             VALUES ('{event_id}', '{project_id}', 'user.signup', 'lifecycle')"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO template (id, project_id, name, channel) \
             VALUES ('{template_id}', '{project_id}', 'welcome', 'email')"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO pipeline_rule (id, event_id, channel, template_id, enabled, priority) \
             VALUES ('{rule_id}', '{event_id}', 'email', '{template_id}', true, 10)"
        ))
        .await
        .unwrap();

        (db, rule_id)
    }

    #[tokio::test]
    async fn crud_middleware() {
        let (db, rule_id) = setup().await;

        let mw_id = Uuid::now_v7();
        let config = serde_json::json!({"key": "value"});

        // Insert
        insert(&db, mw_id, rule_id, "rate_limiter", &config, 10)
            .await
            .unwrap();

        // List (enabled only)
        let rows = list_by_rule(&db, rule_id).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, mw_id);
        assert_eq!(rows[0].middleware_name, "rate_limiter");
        assert_eq!(rows[0].priority, 10);
        assert!(rows[0].enabled);
        assert_eq!(rows[0].config_value().unwrap(), config);

        // Update — disable and change priority
        let new_config = serde_json::json!({"key": "updated"});
        update(&db, mw_id, &new_config, 20, false).await.unwrap();

        // list_by_rule should now return empty (disabled)
        let rows = list_by_rule(&db, rule_id).await.unwrap();
        assert_eq!(rows.len(), 0);

        // list_all_by_rule should still return it
        let rows = list_all_by_rule(&db, rule_id).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].enabled);
        assert_eq!(rows[0].priority, 20);
        assert_eq!(rows[0].config_value().unwrap(), new_config);

        // Delete
        delete(&db, mw_id).await.unwrap();
        let rows = list_all_by_rule(&db, rule_id).await.unwrap();
        assert_eq!(rows.len(), 0);
    }

    #[tokio::test]
    async fn middleware_ordered_by_priority() {
        let (db, rule_id) = setup().await;

        let id_high = Uuid::now_v7();
        let id_low = Uuid::now_v7();
        let config = serde_json::json!({});

        // Insert higher priority first
        insert(&db, id_high, rule_id, "auth", &config, 50)
            .await
            .unwrap();

        // Insert lower priority second
        insert(&db, id_low, rule_id, "logging", &config, 5)
            .await
            .unwrap();

        let rows = list_by_rule(&db, rule_id).await.unwrap();
        assert_eq!(rows.len(), 2);
        // Lower priority number comes first (ASC)
        assert_eq!(rows[0].id, id_low);
        assert_eq!(rows[0].priority, 5);
        assert_eq!(rows[1].id, id_high);
        assert_eq!(rows[1].priority, 50);
    }
}
