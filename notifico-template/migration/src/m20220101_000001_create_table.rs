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
                    .col(string(Template::Channel))
                    .col(json_binary(Template::Template))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("uniq_template_project_id_name")
                    .table(Template::Table)
                    .col(Template::ProjectId)
                    .col(Template::Name)
                    .col(Template::Channel)
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
    Channel,
    Template,
}
