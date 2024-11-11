use crate::error::EngineError;
use crate::http::admin::{ListQueryParams, PaginatedResult};
use crate::pipeline::{Event, Pipeline};
use async_trait::async_trait;
use std::error::Error;
use uuid::Uuid;

pub struct PipelineResult {
    pub pipeline: Pipeline,
    pub events: Vec<Event>,
}

#[async_trait]
pub trait PipelineStorage: Send + Sync {
    async fn get_pipelines_for_event(
        &self,
        project: Uuid,
        event_name: &str,
    ) -> Result<Vec<Pipeline>, EngineError>;
    async fn list_pipelines_with_events(
        &self,
        params: ListQueryParams,
    ) -> Result<(Vec<(Pipeline, Vec<Event>)>, u64), EngineError>;

    async fn list_events(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<Event>, EngineError>;

    async fn get_event_by_id(&self, id: Uuid) -> Result<Option<Event>, Box<dyn Error>>;
    async fn create_event(&self, project_id: Uuid, name: &str) -> Result<Event, Box<dyn Error>>;
    async fn update_event(&self, id: Uuid, name: &str) -> Result<Event, Box<dyn Error>>;
    async fn delete_event(&self, id: Uuid) -> Result<(), Box<dyn Error>>;
}
