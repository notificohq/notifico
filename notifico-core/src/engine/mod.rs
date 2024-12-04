use crate::error::EngineError;
use crate::recipient::Recipient;
use crate::step::SerializedStep;
use crate::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tracing::instrument;
use utoipa::ToSchema;
use uuid::Uuid;

pub mod plugin;

use crate::pipeline::Pipeline;
pub use plugin::{EnginePlugin, StepOutput};

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct EventContext(pub Map<String, Value>);

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub content: RenderedTemplate,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContext {
    pub pipeline: Pipeline,
    pub step_number: usize,

    pub project_id: Uuid,
    pub event_id: Uuid,
    pub notification_id: Uuid,

    pub recipient: Option<Recipient>,

    pub event_name: String,
    pub event_context: EventContext,
    pub plugin_contexts: Map<String, Value>,
    pub messages: Vec<Message>,
}

impl PipelineContext {
    pub fn get_recipient(&self) -> Result<&Recipient, EngineError> {
        self.recipient.as_ref().ok_or(EngineError::RecipientNotSet)
    }
}

/// Engine is used to run steps in the pipeline.
/// Can be cloned and shared across tasks.
#[derive(Clone, Default)]
pub struct Engine {
    steps: HashMap<Cow<'static, str>, Arc<dyn EnginePlugin>>,
}

impl Debug for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let steps = self.steps.keys().cloned().collect::<Vec<_>>();
        f.write_fmt(format_args!("Engine {{ steps: [{:?}] }}", steps))
    }
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_plugin(&mut self, plugin: Arc<dyn EnginePlugin + 'static>) {
        self.steps.extend(
            plugin
                .steps()
                .into_iter()
                .map(|step| (step, plugin.clone())),
        );
    }

    #[instrument]
    pub async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step_type = step.get_type();

        let Some(plugin) = self.steps.get(step_type) else {
            return Err(EngineError::PluginNotFound(step_type.into()));
        };

        plugin.execute_step(context, step).await
    }
}
