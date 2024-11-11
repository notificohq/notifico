use crate::error::EngineError;
use crate::pipeline::Pipeline;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait PipelineStorage: Send + Sync {
    async fn get_pipelines_for_event(
        &self,
        project: Uuid,
        event_name: &str,
    ) -> Result<Vec<Pipeline>, EngineError>;
}
