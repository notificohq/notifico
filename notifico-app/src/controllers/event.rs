use crate::crud_table::{
    AdminCrudError, AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use crate::entity;
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

impl AdminCrudTable for EventDbController {
    type Item = Event;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, AdminCrudError> {
        let model = entity::event::Entity::find_by_id(id).one(&self.db).await?;
        Ok(model.map(|m| m.into()))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, AdminCrudError> {
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

    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, AdminCrudError> {
        let id = Uuid::now_v7();

        entity::event::ActiveModel {
            id: Set(id),
            project_id: Set(item.project_id),
            name: Set(item.name.clone()),
            enabled: Set(item.enabled),
            description: Set(item.description.clone()),
        }
        .insert(&self.db)
        .await?;

        Ok(ItemWithId { id, item })
    }

    async fn update(
        &self,
        id: Uuid,
        item: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, AdminCrudError> {
        entity::event::ActiveModel {
            id: Unchanged(id),
            name: Set(item.name.clone()),
            enabled: Set(item.enabled),
            description: Set(item.description.clone()),
            ..Default::default()
        }
        .update(&self.db)
        .await?;

        Ok(ItemWithId { id, item })
    }

    async fn delete(&self, id: Uuid) -> Result<(), AdminCrudError> {
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
    pub enabled: bool,
    pub description: String,
}

impl From<entity::event::Model> for Event {
    fn from(value: entity::event::Model) -> Self {
        Self {
            project_id: value.project_id,
            name: value.name,
            enabled: value.enabled,
            description: value.description,
        }
    }
}
