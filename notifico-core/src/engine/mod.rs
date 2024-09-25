use crate::engine::plugin::EnginePlugin;
use crate::error::EngineError;
use crate::pipeline::SerializedStep;
use crate::recipient::Recipient;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
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
    pub event_context: EventContext,
    pub plugin_contexts: HashMap<Cow<'static, str>, Value>,
}

#[derive(Clone)]
pub struct Engine {
    plugins: HashMap<Cow<'static, str>, Arc<dyn EnginePlugin>>,
}

impl Debug for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let plugins = self.plugins.keys().cloned().collect::<Vec<_>>();
        f.write_fmt(format_args!("Engine {{ plugins: [{:?}] }}", plugins))
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
        }
    }

    pub fn add_plugin(&mut self, plugin: impl EnginePlugin + 'static) {
        self.plugins
            .insert(plugin.step_namespace(), Arc::new(plugin));
    }

    #[instrument]
    pub async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<(), EngineError> {
        let plugin = self
            .plugins
            .get(step.get_namespace())
            .ok_or_else(|| EngineError::PluginNotFound(step.get_type().into()))?;

        plugin.execute_step(context, step).await?;
        Ok(())
    }
}
