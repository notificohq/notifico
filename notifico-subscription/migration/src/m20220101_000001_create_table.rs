use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Subscription::Table)
                    .if_not_exists()
                    .col(pk_uuid(Subscription::Id))
                    .col(uuid(Subscription::ProjectId))
                    .col(uuid(Subscription::RecipientId))
                    .col(string(Subscription::Event))
                    .col(string(Subscription::Channel))
                    .col(boolean(Subscription::IsSubscribed))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_subscription_project_id")
                    .table(Subscription::Table)
                    .col(Subscription::ProjectId)
                    .col(Subscription::RecipientId)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Subscription::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Subscription {
    Table,
    Id,
    ProjectId,
    Event,
    Channel,
    RecipientId,
    IsSubscribed,
}
