use sea_orm_migration::prelude::*;

use super::m20260303_000001_create_projects::Project;

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
                    .col(ColumnDef::new(Template::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Template::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(Template::Name).string_len(255).not_null())
                    .col(ColumnDef::new(Template::Channel).string_len(64).not_null())
                    .col(
                        ColumnDef::new(Template::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Template::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(Template::Table, Template::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_template_project_name")
                    .table(Template::Table)
                    .col(Template::ProjectId)
                    .col(Template::Name)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TemplateVersion::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TemplateVersion::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(TemplateVersion::TemplateId).uuid().not_null())
                    .col(ColumnDef::new(TemplateVersion::Version).integer().not_null())
                    .col(
                        ColumnDef::new(TemplateVersion::IsCurrent)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(TemplateVersion::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TemplateVersion::Table, TemplateVersion::TemplateId)
                            .to(Template::Table, Template::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_template_version_unique")
                    .table(TemplateVersion::Table)
                    .col(TemplateVersion::TemplateId)
                    .col(TemplateVersion::Version)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(TemplateContent::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(TemplateContent::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(TemplateContent::TemplateVersionId).uuid().not_null())
                    .col(ColumnDef::new(TemplateContent::Locale).string_len(10).not_null())
                    .col(ColumnDef::new(TemplateContent::Body).json_binary().not_null())
                    .col(
                        ColumnDef::new(TemplateContent::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(TemplateContent::Table, TemplateContent::TemplateVersionId)
                            .to(TemplateVersion::Table, TemplateVersion::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_template_content_version_locale")
                    .table(TemplateContent::Table)
                    .col(TemplateContent::TemplateVersionId)
                    .col(TemplateContent::Locale)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TemplateContent::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(TemplateVersion::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Template::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Template {
    Table,
    Id,
    ProjectId,
    Name,
    Channel,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum TemplateVersion {
    Table,
    Id,
    TemplateId,
    Version,
    IsCurrent,
    CreatedAt,
}

#[derive(DeriveIden)]
enum TemplateContent {
    Table,
    Id,
    TemplateVersionId,
    Locale,
    Body,
    UpdatedAt,
}
