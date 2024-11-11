use crate::error::EngineError;
use crate::http::admin::{ListQueryParams, PaginatedResult};
use crate::pipeline::{Event, Pipeline};
use async_trait::async_trait;
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
}
