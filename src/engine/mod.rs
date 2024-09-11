use crate::engine::templater::Templater;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod telegram;
pub mod templater;

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub pipelines: Vec<Pipeline>,
}

#[derive(Serialize, Deserialize)]
pub struct Pipeline {
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Step {
    pub(crate) r#type: String,
}

pub struct Recipient {
    pub(crate) telegram_id: i64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventContext(HashMap<String, Value>);

#[derive(Default)]
pub struct PipelineContext {
    pub recipients: Vec<Recipient>,
    pub event_context: EventContext,
    pub plugin_contexts: HashMap<Cow<'static, str>, Arc<Mutex<dyn Any + Send>>>,
}

pub struct Engine<'a> {
    plugins: HashMap<Cow<'static, str>, Arc<dyn EnginePlugin + Send + Sync>>,
    templater: &'a Templater,
}

impl<'a> Engine<'a> {
    pub fn new(templater: &'a Templater) -> Self {
        Self {
            plugins: Default::default(),
            templater,
        }
    }

    pub fn add_plugin(&mut self, plugin: impl EnginePlugin + Send + Sync + 'static) {
        self.plugins.insert(plugin.step_type(), Arc::new(plugin));
    }

    pub(crate) async fn execute_step(
        &mut self,
        context: &mut PipelineContext,
        step_type: &str,
        step: Value,
    ) -> Result<(), EngineError> {
        for (plugin_type, plugin) in self.plugins.iter() {
            if step_type.starts_with(plugin_type.as_ref()) {
                plugin.execute_step(self, context, step).await?;
                return Ok(());
            }
        }
        Err(EngineError::PluginNotFound(step))
    }

    pub(crate) fn get_templater(&self) -> &Templater {
        self.templater
    }
}

#[derive(Debug)]
pub enum EngineError {
    TemplaterError(templater::TemplaterError),
    PluginNotFound(Value),
}

#[async_trait]
pub trait EnginePlugin {
    async fn execute_step(
        &self,
        engine: &Engine,
        context: &mut PipelineContext,
        step: Value,
    ) -> Result<(), EngineError>;

    fn step_type(&self) -> Cow<'static, str>;
}
