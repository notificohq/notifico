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
    pub recipient: Option<RecipientSelector>,
    pub context: EventContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case", untagged)]
pub enum RecipientSelector {
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

        // Determine the recipient based on the recipient selector
        let recipient = match msg.recipient {
            None => None,
            Some(RecipientSelector::Recipient(recipient)) => Some(recipient),
        };

        // Execute each pipeline in a separate task in parallel
        for pipeline in pipelines {
            let recipient = recipient.clone();
            let event_context = msg.context.clone();
            let event_name = msg.event.to_string();

            let channel = pipeline.channel.clone();

            let contact = recipient
                .clone()
                .map(|r| r.get_primary_contact(&channel))
                .unwrap_or_default();

            let context = PipelineContext {
                step_number: 0,

                project_id: msg.project_id,
                recipient,
                event_name,
                event_context,
                plugin_contexts: Default::default(),
                messages: Default::default(),
                channel,
                contact,
                notification_id: Uuid::now_v7(),
                event_id: msg.id,
            };

            let task = serde_json::to_string(&PipelineTask { pipeline, context }).unwrap();

            self.task_tx.send(task).await.unwrap();
        }
        Ok(())
    }
}
