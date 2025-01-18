use crate::entity;
use async_trait::async_trait;
use notifico_core::error::EngineError;
use notifico_core::pipeline::storage::PipelineStorage;
use notifico_core::pipeline::Pipeline;
use sea_orm::prelude::*;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Deserialize;

pub struct DbPipelineStorage {
    db: DatabaseConnection,
}

impl DbPipelineStorage {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
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
            .inner_join(entity::event::Entity)
            .filter(entity::pipeline::Column::ProjectId.eq(project))
            .filter(entity::event::Column::Name.eq(event_name))
            .all(&self.db)
            .await?;

        models.into_iter().map(|m| m.try_into()).collect()
    }
}

impl TryFrom<entity::pipeline::Model> for Pipeline {
    type Error = EngineError;

    fn try_from(value: entity::pipeline::Model) -> Result<Self, Self::Error> {
        Ok(Self {
            project_id: value.project_id,
            steps: Vec::deserialize(value.steps).map_err(EngineError::InvalidStep)?,
        })
    }
}
