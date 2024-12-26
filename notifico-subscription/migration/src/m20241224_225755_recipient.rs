use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Recipient::Table)
                    .if_not_exists()
                    .col(pk_uuid(Recipient::Id))
                    .col(uuid(Recipient::ProjectId))
                    .col(json_binary(Recipient::Extras))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Contact::Table)
                    .if_not_exists()
                    .col(pk_uuid(Contact::Id))
                    .col(uuid(Contact::RecipientId))
                    .col(string(Contact::Contact))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Contact::Table, Contact::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Group::Table)
                    .if_not_exists()
                    .col(pk_uuid(Group::Id))
                    .col(uuid(Group::ProjectId))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(RecipientGroupJ::Table)
                    .if_not_exists()
                    .col(uuid(RecipientGroupJ::RecipientId))
                    .col(uuid(RecipientGroupJ::GroupId))
                    .foreign_key(
                        ForeignKey::create()
                            .from(RecipientGroupJ::Table, RecipientGroupJ::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(RecipientGroupJ::Table, RecipientGroupJ::GroupId)
                            .to(Group::Table, Group::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .primary_key(
                        Index::create()
                            .primary()
                            .col(RecipientGroupJ::RecipientId)
                            .col(RecipientGroupJ::GroupId),
                    )
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
enum Recipient {
    Table,
    Id,
    ProjectId,
    Extras,
}

#[derive(DeriveIden)]
enum Contact {
    Table,
    Id,
    RecipientId,
    #[allow(clippy::enum_variant_names)]
    Contact,
}

#[derive(DeriveIden)]
enum Group {
    Table,
    Id,
    ProjectId,
}

#[derive(DeriveIden)]
enum RecipientGroupJ {
    Table,
    RecipientId,
    GroupId,
}
