use crate::entity::project;
use sea_orm::prelude::Uuid;
use sea_orm::{ActiveModelTrait, Set};
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Project::Table)
                    .if_not_exists()
                    .col(pk_uuid(Project::Id))
                    .col(string(Project::Name))
                    .to_owned(),
            )
            .await?;

        let db = manager.get_connection();

        project::ActiveModel {
            id: Set(Uuid::nil()),
            name: Set("Default Project".to_string()),
        }
        .insert(db)
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Project::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Project {
    Table,
    Id,
    Name,
}
