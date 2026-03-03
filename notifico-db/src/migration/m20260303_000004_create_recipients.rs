use sea_orm_migration::prelude::*;

use super::m20260303_000001_create_projects::Project;
use super::m20260303_000002_create_events::Event;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Recipient::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Recipient::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Recipient::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(Recipient::ExternalId).string_len(255).not_null())
                    .col(
                        ColumnDef::new(Recipient::Locale)
                            .string_len(10)
                            .not_null()
                            .default("en"),
                    )
                    .col(
                        ColumnDef::new(Recipient::Timezone)
                            .string_len(64)
                            .not_null()
                            .default("UTC"),
                    )
                    .col(
                        ColumnDef::new(Recipient::Metadata)
                            .json_binary()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(Recipient::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Recipient::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Recipient::Table, Recipient::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_recipient_project_external")
                    .table(Recipient::Table)
                    .col(Recipient::ProjectId)
                    .col(Recipient::ExternalId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RecipientContact::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RecipientContact::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RecipientContact::RecipientId).uuid().not_null())
                    .col(ColumnDef::new(RecipientContact::Channel).string_len(64).not_null())
                    .col(ColumnDef::new(RecipientContact::Value).string_len(512).not_null())
                    .col(
                        ColumnDef::new(RecipientContact::Verified)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(RecipientContact::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RecipientContact::Table, RecipientContact::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_recipient_contact_unique")
                    .table(RecipientContact::Table)
                    .col(RecipientContact::RecipientId)
                    .col(RecipientContact::Channel)
                    .col(RecipientContact::Value)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RecipientPreference::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RecipientPreference::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(RecipientPreference::RecipientId).uuid().not_null())
                    .col(
                        ColumnDef::new(RecipientPreference::Category)
                            .string_len(32)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RecipientPreference::Channel)
                            .string_len(64)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RecipientPreference::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(RecipientPreference::ScheduleStart).time().null())
                    .col(ColumnDef::new(RecipientPreference::ScheduleEnd).time().null())
                    .col(
                        ColumnDef::new(RecipientPreference::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RecipientPreference::Table, RecipientPreference::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_recipient_preference_unique")
                    .table(RecipientPreference::Table)
                    .col(RecipientPreference::RecipientId)
                    .col(RecipientPreference::Category)
                    .col(RecipientPreference::Channel)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Unsubscribe::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Unsubscribe::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Unsubscribe::RecipientId).uuid().not_null())
                    .col(ColumnDef::new(Unsubscribe::EventId).uuid().null())
                    .col(ColumnDef::new(Unsubscribe::Category).string_len(32).null())
                    .col(ColumnDef::new(Unsubscribe::Channel).string_len(64).null())
                    .col(
                        ColumnDef::new(Unsubscribe::Token)
                            .string_len(128)
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Unsubscribe::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Unsubscribe::Table, Unsubscribe::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Unsubscribe::Table, Unsubscribe::EventId)
                            .to(Event::Table, Event::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_unsubscribe_recipient")
                    .table(Unsubscribe::Table)
                    .col(Unsubscribe::RecipientId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Unsubscribe::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(RecipientPreference::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(RecipientContact::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Recipient::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Recipient {
    Table,
    Id,
    ProjectId,
    ExternalId,
    Locale,
    Timezone,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum RecipientContact {
    Table,
    Id,
    RecipientId,
    Channel,
    Value,
    Verified,
    CreatedAt,
}

#[derive(DeriveIden)]
enum RecipientPreference {
    Table,
    Id,
    RecipientId,
    Category,
    Channel,
    Enabled,
    ScheduleStart,
    ScheduleEnd,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Unsubscribe {
    Table,
    Id,
    RecipientId,
    EventId,
    Category,
    Channel,
    Token,
    CreatedAt,
}
