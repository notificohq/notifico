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

pub struct SinglePipelineStorage {
    pipeline: Pipeline,
}

impl SinglePipelineStorage {
    pub fn new(pipeline: Pipeline) -> Self {
        Self { pipeline }
    }
}

#[async_trait]
impl PipelineStorage for SinglePipelineStorage {
    async fn get_pipelines_for_event(
        &self,
        _project: Uuid,
        _event_name: &str,
    ) -> Result<Vec<Pipeline>, EngineError> {
        Ok(vec![self.pipeline.clone()])
    }
}
