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
                    .table(Event::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Event::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Event::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(Event::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Event::Category).string_len(32).not_null())
                    .col(ColumnDef::new(Event::Description).text().not_null().default(""))
                    .col(
                        ColumnDef::new(Event::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Event::Table, Event::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_event_project_name")
                    .table(Event::Table)
                    .col(Event::ProjectId)
                    .col(Event::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PipelineRule::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(PipelineRule::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(PipelineRule::EventId).uuid().not_null())
                    .col(ColumnDef::new(PipelineRule::Channel).string_len(64).not_null())
                    .col(ColumnDef::new(PipelineRule::TemplateId).uuid().not_null())
                    .col(ColumnDef::new(PipelineRule::Enabled).boolean().not_null().default(true))
                    .col(ColumnDef::new(PipelineRule::Conditions).json_binary().null())
                    .col(ColumnDef::new(PipelineRule::Priority).integer().not_null().default(0))
                    .foreign_key(
                        ForeignKey::create()
                            .from(PipelineRule::Table, PipelineRule::EventId)
                            .to(Event::Table, Event::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PipelineRule::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Event::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Event {
    Table,
    Id,
    ProjectId,
    Name,
    Category,
    Description,
    CreatedAt,
}

#[derive(DeriveIden)]
enum PipelineRule {
    Table,
    Id,
    EventId,
    Channel,
    TemplateId,
    Enabled,
    Conditions,
    Priority,
}
