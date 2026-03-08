use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260308_000010_create_pipeline_middleware"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE pipeline_middleware (
                    id TEXT PRIMARY KEY,
                    rule_id TEXT NOT NULL REFERENCES pipeline_rule(id) ON DELETE CASCADE,
                    middleware_name TEXT NOT NULL,
                    config TEXT NOT NULL DEFAULT '{}',
                    priority INTEGER NOT NULL DEFAULT 0,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                )",
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_pipeline_middleware_rule_id ON pipeline_middleware(rule_id, priority)",
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS pipeline_middleware")
            .await?;
        Ok(())
    }
}
