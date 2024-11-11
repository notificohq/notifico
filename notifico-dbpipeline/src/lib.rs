use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use notifico_core::error::EngineError;
use notifico_core::http::admin::{ListQueryParams, ListableTrait, PaginatedResult};
use notifico_core::pipeline::storage::{PipelineResult, PipelineStorage};
use notifico_core::pipeline::{Event, Pipeline};
use sea_orm::prelude::Uuid;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, LoaderTrait, PaginatorTrait,
    QueryFilter, QuerySelect, Set,
};
use serde::Deserialize;
use std::error::Error;

mod entity;

pub struct DbPipelineStorage {
    db: DatabaseConnection,
}

impl DbPipelineStorage {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn setup(&self) -> anyhow::Result<()> {
        Ok(Migrator::up(&self.db, None).await?)
    }
}

#[async_trait]
impl PipelineStorage for DbPipelineStorage {
    // For service API. Performance-critical
    async fn get_pipelines_for_event(
        &self,
        project: Uuid,
        event_name: &str,
    ) -> Result<Vec<Pipeline>, EngineError> {
        let models = entity::pipeline::Entity::find()
            .filter(entity::pipeline::Column::ProjectId.eq(project))
            .filter(entity::event::Column::Name.eq(event_name))
            .all(&self.db)
            .await?;

        models.into_iter().map(|m| m.try_into()).collect()
    }

    // For management API
    async fn list_pipelines(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<PipelineResult>, EngineError> {
        let events = entity::pipeline::Entity::find()
            .apply_params(&params)
            .unwrap()
            .find_with_related(entity::event::Entity)
            .all(&self.db)
            .await?;

        let results: Result<Vec<PipelineResult>, EngineError> = events
            .into_iter()
            .map(|(p, e)| {
                Ok(PipelineResult {
                    pipeline: p.try_into()?,
                    event_ids: e.into_iter().map(|e| e.id).collect(),
                })
            })
            .collect();
        let results = results?;

        Ok(PaginatedResult {
            items: results,
            total_count: entity::pipeline::Entity::find()
                .apply_filter(&params)
                .unwrap()
                .count(&self.db)
                .await?,
        })
    }

    async fn get_pipeline_by_id(&self, id: Uuid) -> Result<Option<PipelineResult>, EngineError> {
        let events = entity::pipeline::Entity::find_by_id(id)
            .find_with_related(entity::event::Entity)
            .all(&self.db)
            .await?;

        let results: Result<Vec<PipelineResult>, EngineError> = events
            .into_iter()
            .map(|(p, e)| {
                Ok(PipelineResult {
                    pipeline: p.try_into()?,
                    event_ids: e.into_iter().map(|e| e.id).collect(),
                })
            })
            .collect();
        let results = results?;

        Ok(results.first().cloned())
    }

    async fn list_events(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<Event>, EngineError> {
        Ok(PaginatedResult {
            items: entity::event::Entity::find()
                .apply_params(&params)
                .unwrap()
                .all(&self.db)
                .await?
                .into_iter()
                .map(Event::from)
                .collect(),
            total_count: entity::event::Entity::find()
                .apply_filter(&params)
                .unwrap()
                .count(&self.db)
                .await?,
        })
    }

    async fn get_event_by_id(&self, id: Uuid) -> Result<Option<Event>, Box<dyn Error>> {
        let model = entity::event::Entity::find_by_id(id).one(&self.db).await?;

        Ok(model.map(Event::from))
    }

    async fn create_event(
        &self,
        project_id: Uuid,
        name: &str,
    ) -> Result<Event, Box<dyn std::error::Error>> {
        let id = Uuid::now_v7();

        entity::event::ActiveModel {
            id: Set(id),
            project_id: Set(project_id),
            name: Set(name.to_string()),
        }
        .insert(&self.db)
        .await?;

        Ok(Event {
            id,
            project_id,
            name: name.to_string(),
        })
    }

    async fn update_event(&self, id: Uuid, name: &str) -> Result<Event, Box<dyn Error>> {
        entity::event::ActiveModel {
            id: Set(id),
            name: Set(name.to_string()),
            ..Default::default()
        }
        .update(&self.db)
        .await?;

        Ok(self.get_event_by_id(id).await?.unwrap())
    }

    async fn delete_event(&self, id: Uuid) -> Result<(), Box<dyn Error>> {
        entity::event::ActiveModel {
            id: Set(id),
            ..Default::default()
        }
        .delete(&self.db)
        .await?;
        Ok(())
    }
}

impl TryFrom<entity::pipeline::Model> for Pipeline {
    type Error = EngineError;

    fn try_from(value: entity::pipeline::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            project_id: value.project_id,
            channel: value.channel,
            steps: Vec::deserialize(value.steps).map_err(EngineError::InvalidStep)?,
        })
    }
}

impl From<entity::event::Model> for Event {
    fn from(value: entity::event::Model) -> Self {
        Self {
            id: value.id,
            project_id: value.project_id,
            name: value.name,
        }
    }
}
