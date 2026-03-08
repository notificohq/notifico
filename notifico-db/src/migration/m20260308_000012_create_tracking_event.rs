use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260308_000012_create_tracking_event"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE tracking_event (
                    id TEXT PRIMARY KEY,
                    delivery_log_id TEXT,
                    event_type TEXT NOT NULL,
                    url TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                )",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_tracking_event_delivery ON tracking_event(delivery_log_id)",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS tracking_event")
            .await?;
        Ok(())
    }
}
