use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Template::Table)
                    .if_not_exists()
                    .col(pk_uuid(Template::Id))
                    .col(uuid(Template::ProjectId))
                    .col(string(Template::Name))
                    .col(string(Template::Description))
                    .col(string(Template::Channel))
                    .col(json_binary(Template::Template))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Template::Table, Template::ProjectId)
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
                    .name("uniq_template_project_id_name")
                    .table(Template::Table)
                    .unique()
                    .col(Template::ProjectId)
                    .col(Template::Name)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Template::Table).to_owned())
            .await
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(DeriveIden)]
enum Template {
    Table,
    Id,
    ProjectId,
    Name,
    Description,
    Channel,
    Template,
}

#[derive(DeriveIden)]
enum Project {
    Table,
    Id,
}
