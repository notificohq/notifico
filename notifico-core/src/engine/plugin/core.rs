use crate::engine::{EnginePlugin, PipelineContext, StepOutput};
use crate::error::EngineError;
use crate::pipeline::event::RecipientSelector;
use crate::pipeline::executor::PipelineTask;
use crate::queue::SenderChannel;
use crate::recipient::RecipientController;
use crate::step::SerializedStep;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

pub struct CorePlugin {
    pipeline_sender: Arc<dyn SenderChannel>,
    recipient_controller: Arc<dyn RecipientController>,
}

impl CorePlugin {
    pub fn new(
        pipeline_sender: Arc<dyn SenderChannel>,
        recipient_controller: Arc<dyn RecipientController>,
    ) -> Self {
        Self {
            pipeline_sender,
            recipient_controller,
        }
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
            Step::SetRecipients { recipients } => {
                let mut recipients = self
                    .recipient_controller
                    .get_recipients(context.project_id, recipients)
                    .await?;
                let mut recipient_number = 0;

                while let Some(recipient) = recipients.next().await {
                    recipient_number += 1;
                    if recipient_number == 1 {
                        context.recipient = Some(recipient.clone());
                    } else {
                        let mut context = context.clone();
                        context.step_number += 1;
                        context.recipient = Some(recipient.clone());

                        context.notification_id = Uuid::now_v7();

                        let task = serde_json::to_string(&PipelineTask { context }).unwrap();

                        self.pipeline_sender.send(task).await.unwrap();
                    }
                }
                debug!("Total recipients: {recipient_number}");
                Ok(StepOutput::Continue)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recipient::RecipientInlineController;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_no_recipients() {
        let mut context = PipelineContext::default();
        let step = serde_json::json!({ "step": "core.set_recipients", "recipients": [] });
        let step = SerializedStep(step.as_object().unwrap().clone());

        let (pipeline_tx, pipeline_rx) = flume::unbounded();
        let plugin = CorePlugin::new(Arc::new(pipeline_tx), Arc::new(RecipientInlineController));

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
        let plugin = CorePlugin::new(Arc::new(pipeline_tx), Arc::new(RecipientInlineController));

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
        let plugin = CorePlugin::new(Arc::new(pipeline_tx), Arc::new(RecipientInlineController));

        let output = plugin.execute_step(&mut context, &step).await.unwrap();

        assert_eq!(output, StepOutput::Continue);
        assert_eq!(pipeline_rx.len(), 2)
    }
}
