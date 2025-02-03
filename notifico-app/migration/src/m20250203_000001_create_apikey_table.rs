use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKey::Table)
                    .if_not_exists()
                    .col(pk_uuid(ApiKey::Id))
                    .col(uuid_uniq(ApiKey::Key))
                    .col(uuid(ApiKey::ProjectId))
                    .col(string(ApiKey::Description).default(""))
                    .col(date_time(ApiKey::CreatedAt).default(Expr::current_timestamp()))
                    .foreign_key(
                        ForeignKey::create()
                            .from(ApiKey::Table, ApiKey::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        unimplemented!()
    }
}

#[derive(DeriveIden)]
enum ApiKey {
    Table,
    Id,
    Key,
    ProjectId,
    Description,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Project {
    Table,
    Id,
}
