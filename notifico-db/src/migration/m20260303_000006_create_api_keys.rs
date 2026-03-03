use sea_orm_migration::prelude::*;

use super::m20260303_000001_create_projects::Project;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKey::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ApiKey::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(ApiKey::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(ApiKey::Name).string_len(255).not_null())
                    .col(
                        ColumnDef::new(ApiKey::KeyHash)
                            .string_len(64)
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(ApiKey::KeyPrefix).string_len(16).not_null())
                    .col(ColumnDef::new(ApiKey::Scope).string_len(32).not_null())
                    .col(
                        ColumnDef::new(ApiKey::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(ApiKey::ExpiresAt).timestamp_with_time_zone().null())
                    .col(
                        ColumnDef::new(ApiKey::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(ApiKey::Table, ApiKey::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_api_key_project")
                    .table(ApiKey::Table)
                    .col(ApiKey::ProjectId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKey::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ApiKey {
    Table,
    Id,
    ProjectId,
    Name,
    KeyHash,
    KeyPrefix,
    Scope,
    Enabled,
    ExpiresAt,
    CreatedAt,
}
