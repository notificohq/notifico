use crate::engine::{Engine, StepOutput};
use crate::pipeline::context::PipelineContext;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, instrument};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct PipelineTask {
    pub context: PipelineContext,
}

pub struct PipelineExecutor {
    engine: Engine,
}

impl PipelineExecutor {
    pub fn new(engine: Engine) -> Self {
        Self { engine }
    }

    #[instrument(skip_all)]
    pub async fn execute_pipeline(&self, mut task: PipelineTask) {
        debug!(
            "Executing pipeline: {}",
            serde_json::to_string_pretty(&task.context.pipeline).unwrap()
        );

        let steps = task.context.pipeline.steps.clone();
        for (step_number, step) in steps.iter().enumerate() {
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
