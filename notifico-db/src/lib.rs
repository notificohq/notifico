pub mod migration;
pub mod repo;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

use migration::Migrator;

/// Connect to the database using the provided URL.
pub async fn connect(url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opts = ConnectOptions::new(url);
    opts.sqlx_logging(false);
    Database::connect(opts).await
}

/// Run all pending migrations.
pub async fn run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn migrations_run_on_sqlite() {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        // Verify tables exist by querying sqlite_master
        use sea_orm::{ConnectionTrait, Statement};
        let result = db
            .query_all(Statement::from_string(
                sea_orm::DatabaseBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name",
            ))
            .await
            .unwrap();

        let table_names: Vec<String> = result
            .iter()
            .map(|row| row.try_get::<String>("", "name").unwrap())
            .collect();

        assert!(table_names.contains(&"project".to_string()));
        assert!(table_names.contains(&"event".to_string()));
        assert!(table_names.contains(&"pipeline_rule".to_string()));
        assert!(table_names.contains(&"template".to_string()));
        assert!(table_names.contains(&"template_version".to_string()));
        assert!(table_names.contains(&"template_content".to_string()));
        assert!(table_names.contains(&"recipient".to_string()));
        assert!(table_names.contains(&"recipient_contact".to_string()));
        assert!(table_names.contains(&"recipient_preference".to_string()));
        assert!(table_names.contains(&"unsubscribe".to_string()));
        assert!(table_names.contains(&"delivery_log".to_string()));
        assert!(table_names.contains(&"api_key".to_string()));
    }

    #[tokio::test]
    async fn migrations_are_idempotent() {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();
        // Running again should succeed (if_not_exists)
        run_migrations(&db).await.unwrap();
    }
}
