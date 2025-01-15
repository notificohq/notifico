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
                    .foreign_key(
                        ForeignKey::create()
                            .from(Event::Table, Event::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .unique()
                    .table(Event::Table)
                    .name("idx_u_event_name")
                    .if_not_exists()
                    .col(Event::ProjectId)
                    .col(Event::Name)
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
                    .col(json_binary(Pipeline::Steps))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Pipeline::Table, Pipeline::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
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
    Steps,
}

#[derive(DeriveIden)]
enum PipelineEventJ {
    Table,
    PipelineId,
    EventId,
}

#[derive(DeriveIden)]
enum Project {
    Table,
    Id,
}
