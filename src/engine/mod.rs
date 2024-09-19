use notifico_core::engine::{EnginePlugin, PipelineContext};
use notifico_core::error::EngineError;
use notifico_core::pipeline::{Pipeline, SerializedStep};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tracing::instrument;

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub pipelines: Vec<Pipeline>,
}

pub struct Engine {
    plugins: HashMap<Cow<'static, str>, Arc<dyn EnginePlugin>>,
}

impl Debug for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let plugins = self.plugins.keys().cloned().collect::<Vec<_>>();
        f.write_fmt(format_args!("Engine {{ plugins: [{:?}] }}", plugins))
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
    pub(crate) async fn execute_step(
        &mut self,
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
