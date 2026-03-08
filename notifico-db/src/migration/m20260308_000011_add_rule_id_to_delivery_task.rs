use sea_orm_migration::prelude::*;

use super::m20260304_000008_create_delivery_task::DeliveryTask;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DeliveryTask::Table)
                    .add_column(ColumnDef::new(Alias::new("rule_id")).uuid().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(DeliveryTask::Table)
                    .drop_column(Alias::new("rule_id"))
                    .to_owned(),
            )
            .await
    }
}
