use crate::engine::{EventContext, PipelineContext};
use crate::error::EngineError;
use crate::pipeline::executor::PipelineTask;
use crate::pipeline::storage::PipelineStorage;
use crate::queue::SenderChannel;
use crate::recipient::Recipient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct ProcessEventRequest {
    #[serde(default = "Uuid::now_v7")]
    pub id: Uuid,
    #[serde(default = "Uuid::nil")]
    pub project_id: Uuid,
    pub event: String,
    pub recipients: Vec<RecipientSelector>,
    pub context: EventContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case", untagged)]
pub enum RecipientSelector {
    Recipient(Recipient),
}

impl RecipientSelector {
    pub fn resolve(self) -> Recipient {
        match self {
            RecipientSelector::Recipient(recipient) => recipient,
        }
    }
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

        // Execute each pipeline in a separate task in parallel
        for pipeline in pipelines {
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

            if msg.recipients.is_empty() {
                let task = serde_json::to_string(&PipelineTask { context }).unwrap();
                self.task_tx.send(task).await.unwrap();
                return Ok(());
            }

            for recipient in &msg.recipients {
                let recipient = recipient.clone().resolve();
                let mut context = context.clone();
                context.recipient = Some(recipient.clone());

                let task = serde_json::to_string(&PipelineTask { context }).unwrap();

                self.task_tx.send(task).await.unwrap();
            }
        }
        Ok(())
    }
}
