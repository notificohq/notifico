use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use notifico_core::error::EngineError;
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set};
use serde::{Deserialize, Serialize};
use std::error::Error;
use uuid::Uuid;

#[allow(unused_imports)]
mod entity;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
}

pub struct ProjectController {
    db: DatabaseConnection,
}

impl ProjectController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn setup(&self) -> Result<(), Box<dyn Error>> {
        Ok(Migrator::up(&self.db, None).await?)
    }
}

impl From<entity::project::Model> for Project {
    fn from(value: entity::project::Model) -> Self {
        Project { name: value.name }
    }
}

#[async_trait]
impl AdminCrudTable for ProjectController {
    type Item = Project;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError> {
        let query = entity::project::Entity::find_by_id(id)
            .one(&self.db)
            .await?;
        Ok(query.map(Project::from))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError> {
        let query = entity::project::Entity::find()
            .apply_params(&params)
            .unwrap()
            .all(&self.db)
            .await?;

        Ok(PaginatedResult {
            items: query
                .into_iter()
                .map(|m| ItemWithId {
                    id: m.id,
                    item: Project::from(m),
                })
                .collect(),
            total: entity::project::Entity::find()
                .apply_filter(&params)?
                .count(&self.db)
                .await?,
        })
    }

    async fn create(&self, entity: Self::Item) -> Result<ItemWithId<Self::Item>, EngineError> {
        let id = Uuid::now_v7();

        entity::project::ActiveModel {
            id: Set(id),
            name: Set(entity.name.to_string()),
        }
        .insert(&self.db)
        .await?;

        Ok(ItemWithId {
            id,
            item: Project {
                name: entity.name.to_string(),
            },
        })
    }

    async fn update(
        &self,
        id: Uuid,
        entity: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, EngineError> {
        entity::project::ActiveModel {
            id: Set(id),
            name: Set(entity.name.to_string()),
        }
        .update(&self.db)
        .await?;
        Ok(ItemWithId {
            id,
            item: Project {
                name: entity.name.to_string(),
            },
        })
    }

    async fn delete(&self, id: Uuid) -> Result<(), EngineError> {
        if id.is_nil() {
            return Ok(());
        }

        entity::project::ActiveModel {
            id: Set(id),
            ..Default::default()
        }
        .delete(&self.db)
        .await?;
        Ok(())
    }
}
