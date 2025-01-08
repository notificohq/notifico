use crate::entity;
use async_trait::async_trait;
use notifico_core::error::EngineError;
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::{Set, Unchanged};
use sea_orm::{ActiveModelTrait, PaginatorTrait};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub struct EventDbController {
    db: DatabaseConnection,
}

impl EventDbController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl AdminCrudTable for EventDbController {
    type Item = Event;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError> {
        let model = entity::event::Entity::find_by_id(id).one(&self.db).await?;
        Ok(model.map(|m| m.into()))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError> {
        let params = params.try_into()?;
        Ok(PaginatedResult {
            items: entity::event::Entity::find()
                .apply_params(&params)?
                .all(&self.db)
                .await?
                .into_iter()
                .map(|m| ItemWithId {
                    id: m.id,
                    item: m.into(),
                })
                .collect(),
            total: entity::event::Entity::find()
                .apply_filter(&params)?
                .count(&self.db)
                .await?,
        })
    }

    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, EngineError> {
        let id = Uuid::now_v7();

        entity::event::ActiveModel {
            id: Set(id),
            project_id: Set(item.project_id),
            name: Set(item.name.clone()),
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
        entity::event::ActiveModel {
            id: Unchanged(id),
            name: Set(item.name.clone()),
            ..Default::default()
        }
        .update(&self.db)
        .await?;

        Ok(ItemWithId { id, item })
    }

    async fn delete(&self, id: Uuid) -> Result<(), EngineError> {
        entity::event::ActiveModel {
            id: Set(id),
            ..Default::default()
        }
        .delete(&self.db)
        .await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, ToSchema)]
pub struct Event {
    pub project_id: Uuid,
    pub name: String,
}

impl From<entity::event::Model> for Event {
    fn from(value: entity::event::Model) -> Self {
        Self {
            project_id: value.project_id,
            name: value.name,
        }
    }
}
