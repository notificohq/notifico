use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use notifico_core::error::EngineError;
use notifico_core::http::admin::{ListQueryParams, ListableTrait, PaginatedResult};
use notifico_core::pipeline::storage::PipelineStorage;
use notifico_core::pipeline::{Event, Pipeline};
use sea_orm::prelude::Uuid;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, LoaderTrait, PaginatorTrait,
    QueryFilter, QuerySelect,
};
use serde::Deserialize;

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
    async fn list_pipelines_with_events(
        &self,
        params: ListQueryParams,
    ) -> Result<(Vec<(Pipeline, Vec<Event>)>, u64), EngineError> {
        let mut query_count = entity::pipeline::Entity::find();
        query_count = query_count.apply_filter(&params).unwrap();
        let count = query_count.count(&self.db).await?;

        let mut query = entity::pipeline::Entity::find();
        query = query.apply_params(&params).unwrap();

        //
        // let project = Uuid::nil();
        // let mid = Uuid::now_v7();
        // let m = entity::pipeline::ActiveModel {
        //     id: Set(mid),
        //     project_id: Set(project),
        //     channel: Set("email".to_string()),
        //     steps: Set(json!([])),
        // };
        // m.insert(&self.db).await?;
        //
        // let meid = Uuid::now_v7();
        // let me = entity::event::ActiveModel {
        //     id: Set(meid),
        //     project_id: Set(project),
        //     name: Set("send_email".to_string()),
        // };
        // me.insert(&self.db).await?;
        //
        // let mx = entity::pipeline_event_j::ActiveModel {
        //     pipeline_id: Set(mid),
        //     event_id: Set(meid),
        // };
        // mx.insert(&self.db).await?;
        //
        let events = query
            .find_with_related(entity::event::Entity)
            .all(&self.db)
            .await?;

        let results: Result<Vec<(Pipeline, Vec<Event>)>, EngineError> = events
            .into_iter()
            .map(|(p, e)| Ok((p.try_into()?, e.into_iter().map(Event::from).collect())))
            .collect();

        Ok((results?, count))
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
