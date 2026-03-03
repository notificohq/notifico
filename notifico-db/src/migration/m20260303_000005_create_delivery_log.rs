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
                    .table(DeliveryLog::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(DeliveryLog::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(DeliveryLog::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(DeliveryLog::EventName).string_len(255).not_null())
                    .col(ColumnDef::new(DeliveryLog::RecipientId).uuid().not_null())
                    .col(ColumnDef::new(DeliveryLog::Channel).string_len(64).not_null())
                    .col(
                        ColumnDef::new(DeliveryLog::Status)
                            .string_len(32)
                            .not_null()
                            .default("queued"),
                    )
                    .col(ColumnDef::new(DeliveryLog::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(DeliveryLog::Attempts)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(DeliveryLog::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(DeliveryLog::DeliveredAt).timestamp_with_time_zone().null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(DeliveryLog::Table, DeliveryLog::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(DeliveryLog::Table, DeliveryLog::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_delivery_log_project_created")
                    .table(DeliveryLog::Table)
                    .col(DeliveryLog::ProjectId)
                    .col(DeliveryLog::CreatedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_delivery_log_recipient")
                    .table(DeliveryLog::Table)
                    .col(DeliveryLog::RecipientId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_delivery_log_status")
                    .table(DeliveryLog::Table)
                    .col(DeliveryLog::Status)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DeliveryLog::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum DeliveryLog {
    Table,
    Id,
    ProjectId,
    EventName,
    RecipientId,
    Channel,
    Status,
    ErrorMessage,
    Attempts,
    CreatedAt,
    DeliveredAt,
}
