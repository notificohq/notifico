use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NcenterNotification::Table)
                    .if_not_exists()
                    .col(pk_uuid(NcenterNotification::Id))
                    .col(uuid(NcenterNotification::RecipientId))
                    .col(uuid(NcenterNotification::ProjectId))
                    .col(json_binary(NcenterNotification::Content))
                    .col(date_time(NcenterNotification::CreatedAt))
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_ncenter_notifications_recipient_id")
                    .table(NcenterNotification::Table)
                    .if_not_exists()
                    .col(NcenterNotification::RecipientId)
                    .col(NcenterNotification::ProjectId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NcenterNotification::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum NcenterNotification {
    Table,
    Id,
    RecipientId,
    ProjectId,
    Content,
    CreatedAt,
}
