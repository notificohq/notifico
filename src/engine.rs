use notifico_core::engine::{EnginePlugin, EventContext, PipelineContext};
use notifico_core::error::EngineError;
use notifico_core::pipeline::{Pipeline, PipelineStorage, SerializedStep};
use notifico_core::recipient::Recipient;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tokio::task::JoinSet;
use tracing::instrument;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub pipelines: Vec<Pipeline>,
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

#[derive(Default)]
pub struct PipelineRunner {}

impl PipelineRunner {
    pub async fn process_event(
        &self,
        pipeline_storage: Arc<dyn PipelineStorage>,
        project_id: Uuid,
        event: &str,
        event_context: EventContext,
        engine: Engine,
        recipient: Recipient,
    ) -> Result<(), EngineError> {
        let pipelines = pipeline_storage.get_pipelines(project_id, &event).unwrap();

        let mut join_handles = JoinSet::new();

        // Pipeline;
        for pipeline in pipelines {
            let engine = engine.clone();
            let recipient = recipient.clone();
            let event_context = event_context.clone();
            join_handles.spawn(async move {
                let mut context = PipelineContext::default();
                context.project_id = project_id;
                context.recipient = Some(recipient);
                context.event_context = event_context;

                for step in pipeline.steps.iter() {
                    engine.execute_step(&mut context, step).await.unwrap()
                }
            });
        }

        join_handles.join_all().await;
        Ok(())
    }
}
