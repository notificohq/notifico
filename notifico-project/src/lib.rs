use migration::{Migrator, MigratorTrait};
use notifico_core::http::admin::{ListQueryParams, ListableTrait, PaginatedResult};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set};
use serde::Serialize;
use uuid::Uuid;

mod entity;

#[derive(Clone, Debug, Serialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
}

pub struct ProjectController {
    db: DatabaseConnection,
}

impl ProjectController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn setup(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(Migrator::up(&self.db, None).await?)
    }

    pub async fn create(&self, name: &str) -> Result<Project, Box<dyn std::error::Error>> {
        let id = Uuid::now_v7();

        entity::project::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
        }
        .insert(&self.db)
        .await?;

        Ok(Project {
            id,
            name: name.to_string(),
        })
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<Project>, Box<dyn std::error::Error>> {
        let query = entity::project::Entity::find_by_id(id)
            .one(&self.db)
            .await?;
        Ok(query.map(Project::from))
    }

    pub async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<Project>, Box<dyn std::error::Error>> {
        let query = entity::project::Entity::find()
            .apply_params(&params)?
            .all(&self.db)
            .await?;

        Ok(PaginatedResult {
            items: query.into_iter().map(Project::from).collect(),
            total_count: entity::project::Entity::find().count(&self.db).await?,
        })
    }

    pub async fn update(
        &self,
        id: Uuid,
        name: &str,
    ) -> Result<Project, Box<dyn std::error::Error>> {
        entity::project::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
        }
        .update(&self.db)
        .await?;
        Ok(Project {
            id,
            name: name.to_string(),
        })
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), Box<dyn std::error::Error>> {
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

impl From<entity::project::Model> for Project {
    fn from(value: entity::project::Model) -> Self {
        Project {
            id: value.id,
            name: value.name,
        }
    }
}
