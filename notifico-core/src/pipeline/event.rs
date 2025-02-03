use crate::error::EngineError;
use crate::pipeline::context::{EventContext, PipelineContext};
use crate::pipeline::executor::PipelineTask;
use crate::pipeline::storage::PipelineStorage;
use crate::queue::SenderChannel;
use crate::recipient::Recipient;
use crate::step::SerializedStep;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::warn;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessEventRequest {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event: String,
    pub recipients: Vec<RecipientSelector>,
    pub context: EventContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case", untagged)]
pub enum RecipientSelector {
    Id(Uuid),
    Recipient(Recipient),
}

pub struct EventHandler {
    pipeline_storage: Arc<dyn PipelineStorage>,
    task_tx: Arc<dyn SenderChannel>,
}

impl EventHandler {
    pub fn new(
        pipeline_storage: Arc<dyn PipelineStorage>,
        task_tx: Arc<dyn SenderChannel>,
    ) -> Self {
        Self {
            pipeline_storage,
            task_tx,
        }
    }

    pub async fn process_eventrequest(&self, msg: ProcessEventRequest) -> Result<(), EngineError> {
        // Fetch the pipelines associated with the project and event
        let pipelines = self
            .pipeline_storage
            .get_pipelines_for_event(msg.project_id, &msg.event)
            .await?;

        if pipelines.is_empty() {
            warn!(
                "No pipelines found for project: {}, event: {}",
                msg.project_id, msg.event
            );
        }

        // Execute each pipeline in a separate task in parallel
        for mut pipeline in pipelines {
            if !msg.recipients.is_empty() {
                let step = json!({
                    "step": "core.set_recipients",
                    "recipients": msg.recipients.clone(),
                });
                pipeline
                    .steps
                    .insert(0, SerializedStep(step.as_object().cloned().unwrap()));
            }

            let context = PipelineContext {
                pipeline: pipeline.clone(),
                step_number: 0,

                project_id: msg.project_id,
                recipient: Default::default(),
                event_name: msg.event.clone(),
                event_context: msg.context.clone(),
                plugin_contexts: Default::default(),
                messages: Default::default(),
                notification_id: Uuid::now_v7(),
                event_id: msg.id,
            };

            let task = PipelineTask { context };
            self.task_tx.send(task).await.unwrap();
        }
        Ok(())
    }
}
