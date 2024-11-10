use sea_orm_migration::{prelude::*, schema::*};

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
                    .col(pk_uuid(Event::Id))
                    .col(uuid(Event::ProjectId))
                    .col(string(Event::Name))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Pipeline::Table)
                    .if_not_exists()
                    .col(pk_uuid(Pipeline::Id))
                    .col(uuid(Pipeline::ProjectId))
                    .col(string(Pipeline::Channel))
                    .col(json_binary(Pipeline::Steps))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(PipelineEventJ::Table)
                    .if_not_exists()
                    .col(uuid(PipelineEventJ::PipelineId))
                    .col(uuid(PipelineEventJ::EventId))
                    .foreign_key(
                        ForeignKey::create()
                            .from(PipelineEventJ::Table, PipelineEventJ::PipelineId)
                            .to(Pipeline::Table, Pipeline::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(PipelineEventJ::Table, PipelineEventJ::EventId)
                            .to(Event::Table, Event::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .index(
                        Index::create()
                            .primary()
                            .name("pk_pipeline_event_j")
                            .table(PipelineEventJ::Table)
                            .col(PipelineEventJ::PipelineId)
                            .col(PipelineEventJ::EventId),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PipelineEventJ::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Pipeline::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Event::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Event {
    Table,
    Id,
    ProjectId,
    Name,
}

#[derive(DeriveIden)]
enum Pipeline {
    Table,
    Id,
    ProjectId,
    Channel,
    Steps,
}

#[derive(DeriveIden)]
enum PipelineEventJ {
    Table,
    PipelineId,
    EventId,
}
