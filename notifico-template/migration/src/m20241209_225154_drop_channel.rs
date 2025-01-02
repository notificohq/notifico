use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("uniq_template_project_id_name")
                    .table(Template::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("uniq_template_project_id_name")
                    .table(Template::Table)
                    .col(Template::ProjectId)
                    .col(Template::Name)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        unimplemented!()
    }
}

#[derive(DeriveIden)]
enum Template {
    Table,
    ProjectId,
    Name,
}
