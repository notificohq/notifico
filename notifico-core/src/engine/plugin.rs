use crate::engine::PipelineContext;
use crate::error::EngineError;
use crate::pipeline::SerializedStep;
use async_trait::async_trait;
use std::any::Any;
use std::borrow::Cow;

#[async_trait]
pub trait EnginePlugin: Send + Sync + Any {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<(), EngineError>;

    fn step_namespace(&self) -> Cow<'static, str>;
}
