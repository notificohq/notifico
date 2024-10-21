use crate::error::EngineError;
use crate::pipeline::Pipeline;
use uuid::Uuid;

pub trait PipelineStorage: Send + Sync {
    fn get_pipelines(&self, project: Uuid, event_name: &str) -> Result<Vec<Pipeline>, EngineError>;
}
