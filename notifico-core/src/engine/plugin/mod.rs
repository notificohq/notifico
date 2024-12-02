pub mod core;

use crate::engine::PipelineContext;
use crate::error::EngineError;
use crate::step::SerializedStep;
use async_trait::async_trait;
use std::any::Any;
use std::borrow::Cow;

#[derive(Debug, Eq, PartialEq)]
pub enum StepOutput {
    Continue,
    Interrupt,
}

#[async_trait]
pub trait EnginePlugin: Send + Sync + Any {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError>;

    fn steps(&self) -> Vec<Cow<'static, str>> {
        vec![]
    }
}
