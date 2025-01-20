use crate::entity;
use crate::entity::prelude::*;
use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::{stream, StreamExt};
use notifico_core::error::EngineError;
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use notifico_core::pipeline::event::RecipientSelector;
use notifico_core::recipient::{RawContact, RecipientController};
use sea_orm::ActiveValue::{Set, Unchanged};
use sea_orm::{ActiveModelTrait, DbErr, TransactionTrait};
use sea_orm::{ColumnTrait, PaginatorTrait, QueryFilter};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct RecipientDbController {
    db: DatabaseConnection,
}

impl RecipientDbController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RecipientItem {
    pub extras: HashMap<String, String>,
    pub project_id: Uuid,
    pub group_ids: Vec<Uuid>,
    pub contacts: Vec<RawContact>,
}

#[async_trait]
impl AdminCrudTable for RecipientDbController {
    type Item = RecipientItem;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError> {
        let recipient = Recipient::find_by_id(id).one(&self.db).await?;
        let Some(recipient) = recipient else {
            return Ok(None);
        };

        let group_ids = GroupMembership::find()
            .filter(crate::entity::group_membership::Column::RecipientId.eq(id))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|g| g.group_id)
            .collect();

        let contacts = Contact::find()
            .filter(crate::entity::contact::Column::RecipientId.eq(id))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|m| m.contact.parse().unwrap())
            .collect();

        let extras = serde_json::from_value(recipient.extras).unwrap();

        Ok(Some(RecipientItem {
            group_ids,
            contacts,
            extras,
            project_id: recipient.project_id,
        }))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError> {
        let params = params.try_into()?;
        let total = Recipient::find()
            .apply_filter(&params)?
            .count(&self.db)
            .await?;

        let recipients = Recipient::find()
            .apply_params(&params)?
            .all(&self.db)
            .await?;

        let mut items = vec![];
        for recipient in recipients {
            let group_ids = GroupMembership::find()
                .filter(crate::entity::group_membership::Column::RecipientId.eq(recipient.id))
                .all(&self.db)
                .await?
                .into_iter()
                .map(|g| g.group_id)
                .collect();

            let contacts = Contact::find()
                .filter(crate::entity::contact::Column::RecipientId.eq(recipient.id))
                .all(&self.db)
                .await?
                .into_iter()
                .map(|m| m.contact.parse().unwrap())
                .collect();

            let extras = serde_json::from_value(recipient.extras).unwrap();
            items.push(ItemWithId {
                id: recipient.id,
                item: RecipientItem {
                    project_id: recipient.project_id,
                    group_ids,
                    contacts,
                    extras,
                },
            });
        }

        Ok(PaginatedResult { items, total })
    }

    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, EngineError> {
        let id = Uuid::now_v7();
        crate::entity::recipient::ActiveModel {
            id: Set(id),
            project_id: Set(item.project_id),
            extras: Set(serde_json::to_value(item.extras.clone()).unwrap()),
        }
        .insert(&self.db)
        .await?;

        self.assign_contacts(id, item.contacts.clone()).await?;
        self.assign_groups(id, item.group_ids.clone()).await?;

        Ok(ItemWithId { id, item })
    }

    async fn update(
        &self,
        id: Uuid,
        item: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, EngineError> {
        crate::entity::recipient::ActiveModel {
            id: Unchanged(id),
            project_id: Set(item.project_id),
            extras: Set(serde_json::to_value(item.extras.clone()).unwrap()),
        }
        .update(&self.db)
        .await?;

        self.assign_contacts(id, item.contacts.clone()).await?;
        self.assign_groups(id, item.group_ids.clone()).await?;

        Ok(ItemWithId { id, item })
    }

    async fn delete(&self, id: Uuid) -> Result<(), EngineError> {
        Recipient::delete_by_id(id).exec(&self.db).await?;
        Ok(())
    }
}

impl RecipientDbController {
    pub async fn assign_contacts(
        &self,
        recipient_id: Uuid,
        contacts: Vec<RawContact>,
    ) -> Result<(), EngineError> {
        let current_contacts: HashSet<RawContact> = Contact::find()
            .filter(crate::entity::contact::Column::RecipientId.eq(recipient_id))
            .all(&self.db)
            .await?
            .into_iter()
            .map(|m| m.contact.parse().unwrap())
            .collect();
        let new_contacts: HashSet<RawContact> = contacts.into_iter().collect();

        let to_delete: Vec<RawContact> = current_contacts
            .difference(&new_contacts)
            .cloned()
            .collect();

        if !to_delete.is_empty() {
            Contact::delete_many()
                .filter(crate::entity::contact::Column::RecipientId.eq(recipient_id))
                .filter(
                    crate::entity::contact::Column::Contact
                        .is_in(to_delete.into_iter().map(|c| c.to_string())),
                )
                .exec(&self.db)
                .await?;
        }

        let to_insert: Vec<RawContact> = new_contacts
            .difference(&current_contacts)
            .cloned()
            .collect();

        if !to_insert.is_empty() {
            Contact::insert_many(to_insert.into_iter().map(|c| {
                crate::entity::contact::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    recipient_id: Set(recipient_id),
                    contact: Set(c.to_string()),
                }
            }))
            .exec(&self.db)
            .await?;
        }

        Ok(())
    }

    pub async fn assign_groups(
        &self,
        recipient_id: Uuid,
        group_ids: Vec<Uuid>,
    ) -> Result<(), EngineError> {
        self.db
            .transaction::<_, (), DbErr>(|txn| {
                Box::pin(async move {
                    let current_memberships: HashSet<Uuid> = GroupMembership::find()
                        .filter(
                            crate::entity::group_membership::Column::RecipientId.eq(recipient_id),
                        )
                        .all(txn)
                        .await?
                        .into_iter()
                        .map(|m| m.group_id)
                        .collect();

                    let new_memberships: HashSet<Uuid> = group_ids.into_iter().collect();

                    let to_insert: Vec<Uuid> = new_memberships
                        .difference(&current_memberships)
                        .copied()
                        .collect();

                    if !to_insert.is_empty() {
                        GroupMembership::insert_many(to_insert.into_iter().map(|id| {
                            crate::entity::group_membership::ActiveModel {
                                id: Set(Uuid::now_v7()),
                                group_id: Set(id),
                                recipient_id: Set(recipient_id),
                            }
                        }))
                        .exec(txn)
                        .await?;
                    }

                    let to_delete: Vec<Uuid> = current_memberships
                        .difference(&new_memberships)
                        .copied()
                        .collect();

                    if !to_delete.is_empty() {
                        GroupMembership::delete_many()
                            .filter(
                                crate::entity::group_membership::Column::RecipientId
                                    .eq(recipient_id),
                            )
                            .filter(
                                crate::entity::group_membership::Column::GroupId.is_in(to_delete),
                            )
                            .exec(txn)
                            .await?;
                    }

                    Ok(())
                })
            })
            .await
            .unwrap();

        Ok(())
    }
}

#[async_trait]
impl RecipientController for RecipientDbController {
    async fn get_recipients(
        &self,
        project: Uuid,
        sel: Vec<RecipientSelector>,
    ) -> Result<BoxStream<notifico_core::recipient::Recipient>, EngineError> {
        let mut inlines = vec![];
        let mut ids = vec![];

        for selector in sel {
            match selector {
                RecipientSelector::Id(id) => ids.push(id),
                RecipientSelector::Recipient(inline) => inlines.push(inline),
            }
        }

        // Fetch recipient ids by group id
        let groups = GroupMembership::find()
            .filter(entity::group_membership::Column::GroupId.is_in(ids.clone()))
            .all(&self.db)
            .await?;
        ids.extend(groups.into_iter().map(|g| g.recipient_id));

        // Individual recipient IDs
        let recipients = Recipient::find()
            .find_with_related(Contact)
            .filter(entity::recipient::Column::Id.is_in(ids.clone()))
            .filter(entity::recipient::Column::ProjectId.eq(project))
            .all(&self.db)
            .await?;

        let mut core_recipients = vec![];
        for (recipient, contacts) in recipients {
            let mut core_recipient = notifico_core::recipient::Recipient {
                id: recipient.id,
                contacts: vec![],
            };
            for contact in contacts {
                core_recipient.contacts.push(contact.contact.parse()?);
            }
            core_recipients.push(core_recipient);
        }
        let individual_recipients = stream::iter(core_recipients);

        Ok(stream::iter(inlines).chain(individual_recipients).boxed())
    }
}
