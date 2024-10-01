use crate::engine::plugin::{EnginePlugin, StepOutput};
use crate::error::EngineError;
use crate::pipeline::SerializedStep;
use crate::recipient::Recipient;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tracing::field::debug;
use tracing::instrument;
use uuid::Uuid;

pub mod plugin;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventContext(pub Map<String, Value>);

#[derive(Default, Debug)]
pub struct PipelineContext {
    pub project_id: Uuid,
    pub recipient: Option<Recipient>,
    pub trigger_event: String,
    pub event_context: EventContext,
    pub plugin_contexts: Map<String, Value>,
}

#[derive(Clone)]
pub struct Engine {
    plugins: HashMap<Cow<'static, str>, Arc<dyn EnginePlugin>>,
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
            plugins: Default::default(),
            steps: Default::default(),
        }
    }

    pub fn add_plugin(&mut self, plugin: Arc<dyn EnginePlugin + 'static>) {
        for step in plugin.steps() {
            self.steps.insert(step.clone(), plugin.clone());
        }
    }

    #[instrument]
    pub async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step_type = step.get_type();

        match self.steps.get(step_type) {
            Some(plugin) => plugin.execute_step(context, step).await,
            None => Err(EngineError::PluginNotFound(step_type.into())),
        }
    }
}
