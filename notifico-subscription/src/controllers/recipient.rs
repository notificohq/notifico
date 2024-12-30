use async_trait::async_trait;
use notifico_core::error::EngineError;
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use sea_orm::ActiveModelTrait;
use sea_orm::ActiveValue::Set;
use sea_orm::PaginatorTrait;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
}

impl From<crate::entity::recipient::Model> for RecipientItem {
    fn from(value: crate::entity::recipient::Model) -> Self {
        RecipientItem {
            extras: HashMap::deserialize(value.extras.clone()).unwrap(),
            project_id: value.project_id,
        }
    }
}

impl From<crate::entity::recipient::Model> for ItemWithId<RecipientItem> {
    fn from(value: crate::entity::recipient::Model) -> Self {
        ItemWithId {
            id: value.id,
            item: value.into(),
        }
    }
}

#[async_trait]
impl AdminCrudTable for RecipientDbController {
    type Item = RecipientItem;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError> {
        Ok(crate::entity::recipient::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .map(|m| m.into()))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError> {
        let total = crate::entity::recipient::Entity::find()
            .apply_filter(&params)?
            .count(&self.db)
            .await?;

        let items = crate::entity::recipient::Entity::find()
            .apply_params(&params)?
            .all(&self.db)
            .await?
            .into_iter()
            .map(|m| m.into())
            .collect();

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

        Ok(ItemWithId { id, item })
    }

    async fn update(
        &self,
        id: Uuid,
        item: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, EngineError> {
        todo!()
    }

    async fn delete(&self, id: Uuid) -> Result<(), EngineError> {
        crate::entity::recipient::Entity::delete_by_id(id)
            .exec(&self.db)
            .await?;
        Ok(())
    }
}

// #[async_trait]
// impl RecipientController for SubscriptionController {
//     async fn get_recipients(
//         &self,
//         project: Uuid,
//         sel: Vec<RecipientSelector>,
//     ) -> Result<BoxStream<notifico_core::recipient::Recipient>, EngineError> {
//         let mut inlines = vec![];
//         let mut ids = vec![];
//
//         for selector in sel {
//             match selector {
//                 RecipientSelector::Id(id) => ids.push(id),
//                 RecipientSelector::Recipient(inline) => inlines.push(inline),
//             }
//         }
//
//         // Individual recipient IDs
//         let recipients = Recipient::find()
//             .find_with_related(Contact)
//             .filter(entity::recipient::Column::Id.is_in(ids.clone()))
//             .filter(entity::recipient::Column::ProjectId.eq(project))
//             .all(&self.db)
//             .await?;
//
//         let mut core_recipients = vec![];
//         for (recipient, contacts) in recipients {
//             let mut core_recipient = notifico_core::recipient::Recipient {
//                 id: recipient.id,
//                 contacts: vec![],
//             };
//             for contact in contacts {
//                 core_recipient.contacts.push(contact.contact.parse()?);
//             }
//             core_recipients.push(core_recipient);
//         }
//         let individual_recipients = stream::iter(core_recipients);
//
//         // Recipient Group IDs
//         // let group_
//
//         Ok(stream::iter(inlines).chain(individual_recipients).boxed())
//     }
// }
