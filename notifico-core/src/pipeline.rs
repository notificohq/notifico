use crate::engine::StepOutput;
use crate::engine::{Engine, EventContext, PipelineContext};
use crate::error::EngineError;
use crate::recipient::{Recipient, RecipientDirectory};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::error;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Pipeline {
    pub channel: String,
    pub events: Vec<String>,
    pub steps: Vec<SerializedStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", untagged)]
pub enum RecipientSelector {
    Recipient(Recipient),
    RecipientId(String),
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(transparent)]
pub struct SerializedStep(pub serde_json::Map<String, Value>);

impl SerializedStep {
    pub fn get_type(&self) -> &str {
        self.0["step"].as_str().expect("Step type must be a string")
    }

    fn into_inner(self) -> serde_json::Map<String, Value> {
        self.0
    }

    fn into_value(self) -> Value {
        Value::Object(self.into_inner())
    }

    pub fn convert_step<T>(self) -> Result<T, EngineError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let s =
            serde_json::to_string(&self.into_value()).map_err(|e| EngineError::InvalidStep(e))?;
        Ok(serde_json::from_str(&s).map_err(|e| EngineError::InvalidStep(e))?)
    }
}

pub trait PipelineStorage: Send + Sync {
    fn get_pipelines(&self, project: Uuid, event_name: &str) -> Result<Vec<Pipeline>, EngineError>;
}

pub struct PipelineRunner {
    pipeline_storage: Arc<dyn PipelineStorage>,
    engine: Engine,
    recipient_storage: Arc<dyn RecipientDirectory>,
}

impl PipelineRunner {
    pub fn new(
        pipeline_storage: Arc<dyn PipelineStorage>,
        engine: Engine,
        recipient_storage: Arc<dyn RecipientDirectory>,
    ) -> Self {
        Self {
            pipeline_storage,
            engine,
            recipient_storage,
        }
    }

    /// Processes an event by executing the associated pipelines.
    ///
    /// # Parameters
    ///
    /// * `project_id` - The unique identifier of the project associated with the event.
    /// * `trigger_event` - The name of the event that triggered the pipeline execution.
    /// * `event_context` - The contextual information related to the event.
    /// * `recipient_sel` - An optional selector for the recipient of the event.
    pub async fn process_event(
        &self,
        project_id: Uuid,
        trigger_event: &str,
        event_context: EventContext,
        recipient_sel: Option<RecipientSelector>,
    ) -> Result<(), EngineError> {
        // Fetch the pipelines associated with the project and event
        let pipelines = self
            .pipeline_storage
            .get_pipelines(project_id, trigger_event)?;

        // Determine the recipient based on the recipient selector
        let recipient = match recipient_sel {
            None => None,
            Some(RecipientSelector::RecipientId(id)) => {
                self.recipient_storage.get_recipient(project_id, &id).await
            }
            Some(RecipientSelector::Recipient(recipient)) => Some(recipient),
        };

        // Execute each pipeline in a separate task in parallel
        let mut join_handles = JoinSet::new();
        for pipeline in pipelines {
            let engine = self.engine.clone();
            let recipient = recipient.clone();
            let event_context = event_context.clone();
            let trigger_event = trigger_event.to_string();

            join_handles.spawn(async move {
                let mut context = PipelineContext {
                    project_id,
                    recipient,
                    trigger_event,
                    event_context,
                    plugin_contexts: Default::default(),
                    messages: Default::default(),
                    channel: pipeline.channel,
                };

                // Execute each step in the pipeline
                for step in pipeline.steps.iter() {
                    let result = engine.execute_step(&mut context, step).await;
                    match result {
                        Ok(StepOutput::Continue) => continue,
                        Ok(StepOutput::Interrupt) => break,
                        Err(err) => {
                            error!("Error executing step: {:?}", err);
                            break;
                        }
                    }
                }
            });
        }

        // Wait for all pipelines to complete
        join_handles.join_all().await;
        Ok(())
    }
}
