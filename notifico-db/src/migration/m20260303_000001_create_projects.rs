use sea_orm_migration::prelude::*;

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
                    .col(ColumnDef::new(Project::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Project::Name).string_len(255).not_null())
                    .col(
                        ColumnDef::new(Project::DefaultLocale)
                            .string_len(10)
                            .not_null()
                            .default("en"),
                    )
                    .col(ColumnDef::new(Project::Settings).json_binary().not_null().default("{}"))
                    .col(
                        ColumnDef::new(Project::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Project::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Project::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Project {
    Table,
    Id,
    Name,
    DefaultLocale,
    Settings,
    CreatedAt,
    UpdatedAt,
}
