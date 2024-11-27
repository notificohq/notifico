use crate::engine::{EnginePlugin, PipelineContext, StepOutput};
use crate::error::EngineError;
use crate::pipeline::runner::RecipientSelector;
use crate::step::SerializedStep;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "core.set_recipient")]
    SetRecipient { recipient: RecipientSelector },
}

pub const STEPS: &[&str] = &["core.set_recipient"];

struct CorePlugin {}

#[async_trait]
impl EnginePlugin for CorePlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::SetRecipient { recipient } => {
                let recipient = match recipient {
                    RecipientSelector::Recipient(r) => r,
                };
                context.recipient = Some(recipient);
                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}
