use async_trait::async_trait;
use notifico_core::error::EngineError;
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use sea_orm::ActiveValue::{Set, Unchanged};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct ContactDbController {
    db: DatabaseConnection,
}

impl ContactDbController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContactItem {
    recipient_id: Uuid,
    contact: String,
}

impl From<crate::entity::contact::Model> for ContactItem {
    fn from(value: crate::entity::contact::Model) -> Self {
        ContactItem {
            recipient_id: value.recipient_id,
            contact: value.contact,
        }
    }
}

impl From<crate::entity::contact::Model> for ItemWithId<ContactItem> {
    fn from(value: crate::entity::contact::Model) -> Self {
        ItemWithId {
            id: value.id,
            item: value.into(),
        }
    }
}

#[async_trait]
impl AdminCrudTable for ContactDbController {
    type Item = ContactItem;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError> {
        Ok(crate::entity::contact::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .map(|m| m.into()))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError> {
        let params = params.try_into()?;
        let items = crate::entity::contact::Entity::find()
            .apply_params(&params)?
            .all(&self.db)
            .await?
            .into_iter()
            .map(|m| m.into())
            .collect();

        Ok(PaginatedResult {
            items,
            total: crate::entity::contact::Entity::find()
                .apply_filter(&params)?
                .count(&self.db)
                .await?,
        })
    }

    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, EngineError> {
        let id = Uuid::now_v7();
        crate::entity::contact::ActiveModel {
            id: Set(id),
            recipient_id: Set(item.recipient_id),
            contact: Set(item.contact.clone()),
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
        crate::entity::contact::ActiveModel {
            id: Unchanged(id),
            recipient_id: Set(item.recipient_id),
            contact: Set(item.contact.clone()),
        }
        .update(&self.db)
        .await?;
        Ok(ItemWithId { id, item })
    }

    async fn delete(&self, id: Uuid) -> Result<(), EngineError> {
        crate::entity::contact::Entity::delete_by_id(id)
            .exec(&self.db)
            .await?;
        Ok(())
    }
}
