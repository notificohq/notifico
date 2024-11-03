use crate::error::EngineError;
use crate::recipient::Recipient;
use crate::step::SerializedStep;
use crate::templater::RenderResponse;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tracing::instrument;
use utoipa::ToSchema;
use uuid::Uuid;

mod plugin;
pub use plugin::{EnginePlugin, StepOutput};

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct EventContext(pub Map<String, Value>);

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PipelineContext {
    pub project_id: Uuid,
    pub recipient: Option<Recipient>,
    pub trigger_event: String,
    pub event_context: EventContext,
    pub plugin_contexts: Map<String, Value>,
    pub messages: Vec<RenderResponse>,
    pub channel: String,
}

/// Engine is used to run steps in the pipeline.
/// Can be cloned and shared across tasks.
#[derive(Clone)]
pub struct Engine {
    steps: HashMap<Cow<'static, str>, Arc<dyn EnginePlugin>>,
}

impl Debug for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let steps = self.steps.keys().cloned().collect::<Vec<_>>();
        f.write_fmt(format_args!("Engine {{ steps: [{:?}] }}", steps))
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            steps: Default::default(),
        }
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
