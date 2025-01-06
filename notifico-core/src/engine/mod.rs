use crate::error::EngineError;
use crate::pipeline::context::PipelineContext;
use crate::step::SerializedStep;
pub use plugin::{EnginePlugin, StepOutput};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use tracing::{debug, debug_span, instrument, Instrument};

pub mod plugin;

/// Engine is used to run steps in the pipeline.
/// Can be cloned and shared across tasks.
#[derive(Clone)]
pub struct Engine {
    steps: HashMap<Cow<'static, str>, Arc<dyn EnginePlugin>>,
}

impl Debug for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Engine"))
    }
}

impl Engine {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        debug!("Creating new Engine instance");
        Self {
            steps: HashMap::new(),
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

    #[instrument(skip_all)]
    pub async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step_type = step.get_type();

        let Some(plugin) = self.steps.get(step_type) else {
            return Err(EngineError::PluginNotFound(step_type.into()));
        };

        let span = debug_span!("execute_step", step = %step_type);

        async move {
            debug!("Starting step");
            let result = plugin.execute_step(context, step).await;
            debug!("Finished step");
            result
        }
        .instrument(span)
        .await
    }
}
