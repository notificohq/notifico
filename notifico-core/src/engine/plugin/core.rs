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
use tracing::debug;
use uuid::Uuid;

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
        let step: Step = step.convert_step()?;

        match step {
            Step::SetRecipients { recipients } => match recipients.len() {
                0 => Ok(StepOutput::Continue),
                1 => {
                    debug!("Single recipient; no fork");
                    let recipient = &recipients[0].clone().resolve();
                    context.recipient = Some(recipient.clone());
                    Ok(StepOutput::Continue)
                }
                n => {
                    debug!("Multiple recipients: {n}; fork");
                    for recipient in recipients {
                        let recipient = recipient.resolve();

                        let mut context = context.clone();

                        context.step_number += 1;
                        context.recipient = Some(recipient.clone());

                        context.notification_id = Uuid::now_v7();

                        let task = serde_json::to_string(&PipelineTask { context }).unwrap();

                        self.pipeline_sender.send(task).await.unwrap();
                    }

                    Ok(StepOutput::Interrupt)
                }
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_no_recipients() {
        let mut context = PipelineContext::default();
        let step = serde_json::json!({ "step": "core.set_recipients", "recipients": [] });
        let step = SerializedStep(step.as_object().unwrap().clone());

        let (pipeline_tx, pipeline_rx) = flume::unbounded();
        let plugin = CorePlugin::new(Arc::new(pipeline_tx));

        let output = plugin.execute_step(&mut context, &step).await.unwrap();
        assert_eq!(output, StepOutput::Continue);
        assert!(pipeline_rx.is_empty())
    }

    #[tokio::test]
    async fn test_single_recipient() {
        let mut context = PipelineContext::default();

        let step = serde_json::json!(
            {
                "step": "core.set_recipients",
                "recipients": [
                    {
                        "id": Uuid::now_v7(),
                        "contacts": [
                            "abc:1234567890"
                        ]
                    }
                ]
            }
        );
        let step = SerializedStep(step.as_object().unwrap().clone());

        let (pipeline_tx, pipeline_rx) = flume::unbounded();
        let plugin = CorePlugin::new(Arc::new(pipeline_tx));

        let output = plugin.execute_step(&mut context, &step).await.unwrap();

        assert_eq!(output, StepOutput::Continue);
        assert!(context.recipient.is_some());
        assert!(pipeline_rx.is_empty())
    }

    #[tokio::test]
    async fn test_many_recipients() {
        let mut context = PipelineContext::default();

        let step = serde_json::json!(
            {
                "step": "core.set_recipients",
                "recipients": [
                    {
                        "id": Uuid::now_v7(),
                        "contacts": [
                            "abc:1234567890"
                        ]
                    },
                    {
                        "id": Uuid::now_v7(),
                        "contacts": [
                            "abc:1234567890"
                        ]
                    }
                ]
            }
        );
        let step = SerializedStep(step.as_object().unwrap().clone());

        let (pipeline_tx, pipeline_rx) = flume::unbounded();
        let plugin = CorePlugin::new(Arc::new(pipeline_tx));

        let output = plugin.execute_step(&mut context, &step).await.unwrap();

        assert_eq!(output, StepOutput::Interrupt);
        assert_eq!(pipeline_rx.len(), 2)
    }
}
