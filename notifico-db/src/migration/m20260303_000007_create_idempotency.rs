use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(IdempotencyRecord::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(IdempotencyRecord::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(IdempotencyRecord::IdempotencyKey)
                            .string_len(512)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IdempotencyRecord::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_idempotency_key_unique")
                    .table(IdempotencyRecord::Table)
                    .col(IdempotencyRecord::IdempotencyKey)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(IdempotencyRecord::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum IdempotencyRecord {
    Table,
    Id,
    IdempotencyKey,
    CreatedAt,
}
