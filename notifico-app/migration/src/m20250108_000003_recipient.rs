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
                    .foreign_key(
                        ForeignKey::create()
                            .from(Recipient::Table, Recipient::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
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
                    .col(string(Group::Name))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Group::Table, Group::ProjectId)
                            .to(Project::Table, Project::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Group::Table)
                    .unique()
                    .name("uniq_group_project_id_name")
                    .col(Group::ProjectId)
                    .col(Group::Name)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GroupMembership::Table)
                    .if_not_exists()
                    .col(pk_uuid(GroupMembership::Id))
                    .col(uuid(GroupMembership::GroupId))
                    .col(uuid(GroupMembership::RecipientId))
                    .foreign_key(
                        ForeignKey::create()
                            .from(GroupMembership::Table, GroupMembership::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(GroupMembership::Table, GroupMembership::GroupId)
                            .to(Group::Table, Group::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Subscription::Table)
                    .if_not_exists()
                    .col(pk_uuid(Subscription::Id))
                    .col(uuid(Subscription::RecipientId))
                    .col(string(Subscription::Event))
                    .col(string(Subscription::Channel))
                    .col(boolean(Subscription::IsSubscribed))
                    .foreign_key(
                        ForeignKey::create()
                            .from(Subscription::Table, Subscription::RecipientId)
                            .to(Recipient::Table, Recipient::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Restrict),
                    )
                    .index(
                        Index::create()
                            .name("idx_subscription_project_id")
                            .table(Subscription::Table)
                            .col(Subscription::RecipientId)
                            .unique(),
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
    Name,
}

#[derive(DeriveIden)]
enum GroupMembership {
    Table,
    Id,
    RecipientId,
    GroupId,
}

#[derive(DeriveIden)]
enum Subscription {
    Table,
    Id,
    Event,
    Channel,
    RecipientId,
    IsSubscribed,
}

#[derive(DeriveIden)]
enum Project {
    Table,
    Id,
}
