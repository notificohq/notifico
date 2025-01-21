use crate::crud_table::{
    AdminCrudError, AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use crate::entity;
use async_trait::async_trait;
use notifico_core::error::EngineError;
use notifico_core::pipeline::storage::PipelineStorage;
use notifico_core::pipeline::Pipeline;
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::{Set, Unchanged};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub struct PipelineDbController {
    db: DatabaseConnection,
}

impl PipelineDbController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

impl PipelineDbController {
    pub async fn assign_events_to_pipeline(
        &self,
        pipeline_id: Uuid,
        event_id: Vec<Uuid>,
    ) -> Result<(), AdminCrudError> {
        let current_events = entity::pipeline_event_j::Entity::find()
            .filter(entity::pipeline_event_j::Column::PipelineId.eq(pipeline_id))
            .all(&self.db)
            .await?;

        let current_ids: HashSet<Uuid> = current_events.into_iter().map(|e| e.event_id).collect();
        let new_ids: HashSet<Uuid> = event_id.into_iter().collect();

        let to_delete: Vec<Uuid> = current_ids.difference(&new_ids).cloned().collect();
        let to_add: Vec<Uuid> = new_ids.difference(&current_ids).cloned().collect();

        if !to_delete.is_empty() {
            entity::pipeline_event_j::Entity::delete_many()
                .filter(entity::pipeline_event_j::Column::PipelineId.eq(pipeline_id))
                .filter(entity::pipeline_event_j::Column::EventId.is_in(to_delete))
                .exec(&self.db)
                .await?;
        }

        if !to_add.is_empty() {
            let to_add_am: Vec<entity::pipeline_event_j::ActiveModel> = to_add
                .into_iter()
                .map(|event_id| entity::pipeline_event_j::ActiveModel {
                    pipeline_id: Set(pipeline_id),
                    event_id: Set(event_id),
                })
                .collect();

            entity::pipeline_event_j::Entity::insert_many(to_add_am)
                .exec(&self.db)
                .await?;
        }

        Ok(())
    }
}

impl AdminCrudTable for PipelineDbController {
    type Item = PipelineItem;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, AdminCrudError> {
        let events = entity::pipeline::Entity::find_by_id(id)
            .find_with_related(entity::event::Entity)
            .all(&self.db)
            .await?;

        let results: Result<Vec<PipelineItem>, EngineError> = events
            .into_iter()
            .map(|(p, e)| {
                Ok(PipelineItem {
                    pipeline: p.try_into()?,
                    event_ids: e.into_iter().map(|e| e.id).collect(),
                    enabled: true, // Assume all pipelines are enabled for now
                })
            })
            .collect();
        let results = results?;

        Ok(results.first().cloned())
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, AdminCrudError> {
        let params = params.try_into()?;
        let pipelines = entity::pipeline::Entity::find()
            .apply_params(&params)
            .unwrap()
            .find_with_related(entity::event::Entity)
            .all(&self.db)
            .await?;

        let results: Result<Vec<ItemWithId<PipelineItem>>, AdminCrudError> = pipelines
            .into_iter()
            .map(|(p, e)| {
                Ok(ItemWithId {
                    id: p.id,
                    item: PipelineItem {
                        pipeline: p.try_into().map_err(|e| AdminCrudError::from(e))?,
                        event_ids: e.into_iter().map(|e| e.id).collect(),
                        enabled: true, // Assume all pipelines are enabled for now
                    },
                })
            })
            .collect();
        let results = results?;

        Ok(PaginatedResult {
            items: results,
            total: entity::pipeline::Entity::find()
                .apply_filter(&params)?
                .count(&self.db)
                .await?,
        })
    }

    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, AdminCrudError> {
        let id = Uuid::now_v7();
        entity::pipeline::ActiveModel {
            id: Set(id),
            project_id: Set(item.pipeline.project_id),
            steps: Set(serde_json::to_value(item.pipeline.steps.clone()).unwrap()),
        }
        .insert(&self.db)
        .await?;

        self.assign_events_to_pipeline(id, item.event_ids.clone())
            .await?;

        Ok(ItemWithId { id, item })
    }

    async fn update(
        &self,
        id: Uuid,
        item: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, AdminCrudError> {
        entity::pipeline::ActiveModel {
            id: Unchanged(id),
            project_id: Set(item.pipeline.project_id),
            steps: Set(serde_json::to_value(item.pipeline.steps.clone()).unwrap()),
        }
        .update(&self.db)
        .await?;

        self.assign_events_to_pipeline(id, item.event_ids.clone())
            .await?;

        Ok(ItemWithId { id, item })
    }

    async fn delete(&self, id: Uuid) -> Result<(), AdminCrudError> {
        entity::pipeline::Entity::delete_by_id(id)
            .exec(&self.db)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl PipelineStorage for PipelineDbController {
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
            .await
            .map_err(|e| anyhow::Error::new(e))?;

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

#[derive(Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    #[serde(flatten)]
    pub pipeline: Pipeline,
    pub event_ids: Vec<Uuid>,
    pub enabled: bool,
}
