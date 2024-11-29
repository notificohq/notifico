use crate::engine::{Engine, PipelineContext, StepOutput};
use crate::pipeline::Pipeline;
use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct PipelineTask {
    pub pipeline: Pipeline,
    pub context: PipelineContext,
}

pub struct PipelineExecutor {
    engine: Engine,
}

impl PipelineExecutor {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    pub async fn execute_pipeline(&self, mut task: PipelineTask) {
        for (step_number, step) in task.pipeline.steps.iter().enumerate() {
            if step_number < task.context.step_number {
                continue;
            }

            let result = self.engine.execute_step(&mut task.context, step).await;
            match result {
                Ok(StepOutput::Continue) => task.context.step_number += 1,
                Ok(StepOutput::Interrupt) => break,
                Err(err) => {
                    error!("Error executing step: {:?}", err);
                    break;
                }
            }
        }
    }
}
