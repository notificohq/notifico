use crate::engine::{EnginePlugin, PipelineContext, StepOutput};
use crate::error::EngineError;
use crate::pipeline::event::RecipientSelector;
use crate::pipeline::executor::PipelineTask;
use crate::queue::SenderChannel;
use crate::step::SerializedStep;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

pub struct CorePlugin {
    pipeline_sender: Arc<dyn SenderChannel>,
}

impl CorePlugin {
    pub fn new(pipeline_sender: Arc<dyn SenderChannel>) -> Self {
        Self { pipeline_sender }
    }
}

#[async_trait]
impl EnginePlugin for CorePlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::SetRecipients { recipients } => {
                if recipients.is_empty() {
                    return Ok(StepOutput::Continue);
                }

                // Special case: if there's only one recipient with single contact, use it directly in current pipeline.
                if recipients.len() == 1 {
                    let recipient = &recipients[0].clone().resolve();

                    let contacts = recipient.get_all_contacts(&context.pipeline.channel);
                    if contacts.len() == 1 {
                        context.recipient = Some(recipient.clone());
                        context.contact = Some(contacts[0].clone());
                        return Ok(StepOutput::Continue);
                    }
                }

                for recipient in recipients {
                    let recipient = recipient.resolve();

                    for contact in recipient.get_all_contacts(&context.pipeline.channel) {
                        let mut context = context.clone();

                        context.step_number += 1;
                        context.recipient = Some(recipient.clone());
                        context.contact = Some(contact);

                        let task = serde_json::to_string(&PipelineTask { context }).unwrap();

                        self.pipeline_sender.send(task).await.unwrap();
                    }
                }

                Ok(StepOutput::Interrupt)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "core.set_recipients")]
    SetRecipients { recipients: Vec<RecipientSelector> },
}

pub(crate) const STEPS: &[&str] = &["core.set_recipients"];
