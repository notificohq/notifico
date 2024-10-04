use crate::engine::plugin::StepOutput;
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RecipientSelector {
    Recipient(Recipient),
    RecipientId { id: Uuid },
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

    pub async fn process_event(
        &self,
        project_id: Uuid,
        trigger_event: &str,
        event_context: EventContext,
        recipient_sel: Option<RecipientSelector>,
    ) -> Result<(), EngineError> {
        let pipelines = self
            .pipeline_storage
            .get_pipelines(project_id, trigger_event)?;

        let mut join_handles = JoinSet::new();

        let recipient = match recipient_sel {
            None => None,
            Some(RecipientSelector::RecipientId { id }) => {
                self.recipient_storage.get_recipient(id).await
            }
            Some(RecipientSelector::Recipient(recipient)) => Some(recipient),
        };

        // Pipeline;
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

                for step in pipeline.steps.iter() {
                    let result = engine.execute_step(&mut context, step).await;
                    match result {
                        Ok(StepOutput::None) => continue,
                        Ok(StepOutput::Interrupt) => break,
                        Err(err) => {
                            error!("Error executing step: {:?}", err);
                            break;
                        }
                    }
                }
            });
        }

        join_handles.join_all().await;
        Ok(())
    }
}
