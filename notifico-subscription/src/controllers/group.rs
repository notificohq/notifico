use async_trait::async_trait;
use notifico_core::error::EngineError;
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use sea_orm::ActiveValue::{Set, Unchanged};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub struct GroupDbController {
    db: DatabaseConnection,
}

impl GroupDbController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GroupItem {
    project_id: Uuid,
    name: String,
}

impl From<crate::entity::group::Model> for GroupItem {
    fn from(value: crate::entity::group::Model) -> Self {
        Self {
            project_id: value.project_id,
            name: value.name,
        }
    }
}

impl From<crate::entity::group::Model> for ItemWithId<GroupItem> {
    fn from(value: crate::entity::group::Model) -> Self {
        ItemWithId {
            id: value.id,
            item: value.into(),
        }
    }
}

#[async_trait]
impl AdminCrudTable for GroupDbController {
    type Item = GroupItem;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError> {
        Ok(crate::entity::group::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .map(|m| m.into()))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError> {
        let params = params.try_into()?;
        let items = crate::entity::group::Entity::find()
            .apply_params(&params)?
            .all(&self.db)
            .await?
            .into_iter()
            .map(|m| m.into())
            .collect();

        Ok(PaginatedResult {
            items,
            total: crate::entity::group::Entity::find()
                .apply_filter(&params)?
                .count(&self.db)
                .await?,
        })
    }

    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, EngineError> {
        let id = Uuid::now_v7();
        crate::entity::group::ActiveModel {
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
        crate::entity::group::ActiveModel {
            id: Unchanged(id),
            project_id: Set(item.project_id),
            name: Set(item.name.clone()),
        }
        .update(&self.db)
        .await?;
        Ok(ItemWithId { id, item })
    }

    async fn delete(&self, id: Uuid) -> Result<(), EngineError> {
        crate::entity::group::Entity::delete_by_id(id)
            .exec(&self.db)
            .await?;
        Ok(())
    }
}
