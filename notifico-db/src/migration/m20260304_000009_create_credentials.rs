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
                    .table(Credential::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Credential::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Credential::ProjectId).uuid().not_null())
                    .col(
                        ColumnDef::new(Credential::Name)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Credential::Channel)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Credential::EncryptedData)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Credential::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(Credential::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Credential::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Credential::Table, Credential::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Unique constraint: one credential name per project
        manager
            .create_index(
                Index::create()
                    .name("idx_credential_project_name")
                    .table(Credential::Table)
                    .col(Credential::ProjectId)
                    .col(Credential::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Index for looking up credentials by project + channel
        manager
            .create_index(
                Index::create()
                    .name("idx_credential_project_channel")
                    .table(Credential::Table)
                    .col(Credential::ProjectId)
                    .col(Credential::Channel)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Credential::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Credential {
    Table,
    Id,
    ProjectId,
    Name,
    Channel,
    EncryptedData,
    Enabled,
    CreatedAt,
    UpdatedAt,
}
