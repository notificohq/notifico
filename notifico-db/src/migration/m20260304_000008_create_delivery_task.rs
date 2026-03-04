use sea_orm_migration::prelude::*;

use super::m20260303_000001_create_projects::Project;
use super::m20260303_000004_create_recipients::Recipient;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DeliveryTask::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DeliveryTask::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DeliveryTask::ProjectId).uuid().not_null())
                    .col(
                        ColumnDef::new(DeliveryTask::EventName)
                            .string_len(255)
                            .not_null(),
                    )
                    .col(ColumnDef::new(DeliveryTask::RecipientId).uuid().not_null())
                    .col(
                        ColumnDef::new(DeliveryTask::Channel)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::ContactValue)
                            .string_len(512)
                            .not_null(),
                    )
                    .col(ColumnDef::new(DeliveryTask::RenderedBody).json().not_null())
                    .col(
                        ColumnDef::new(DeliveryTask::IdempotencyKey)
                            .string_len(512)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::Status)
                            .string_len(32)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::Attempt)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::MaxAttempts)
                            .integer()
                            .not_null()
                            .default(5),
                    )
                    .col(ColumnDef::new(DeliveryTask::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(DeliveryTask::NextRetryAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(DeliveryTask::Table, DeliveryTask::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(DeliveryTask::Table, DeliveryTask::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Index for worker polling: pending tasks ordered by next_retry_at
        manager
            .create_index(
                Index::create()
                    .name("idx_delivery_task_poll")
                    .table(DeliveryTask::Table)
                    .col(DeliveryTask::Status)
                    .col(DeliveryTask::NextRetryAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_delivery_task_project")
                    .table(DeliveryTask::Table)
                    .col(DeliveryTask::ProjectId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DeliveryTask::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum DeliveryTask {
    Table,
    Id,
    ProjectId,
    EventName,
    RecipientId,
    Channel,
    ContactValue,
    RenderedBody,
    IdempotencyKey,
    Status,
    Attempt,
    MaxAttempts,
    ErrorMessage,
    NextRetryAt,
    CreatedAt,
    UpdatedAt,
}
