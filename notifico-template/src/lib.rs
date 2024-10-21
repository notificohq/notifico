mod descriptor;

use crate::descriptor::{Descriptor, TemplateSourceSelector};
use async_trait::async_trait;
use minijinja::Environment;
use notifico_core::engine::{EnginePlugin, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::step::SerializedStep;
use notifico_core::templater::RenderResponse;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use tracing::info;

pub struct LocalTemplater {
    basepath: PathBuf,
    env: Environment<'static>,
}

impl LocalTemplater {
    pub fn new(basepath: &Path) -> Self {
        Self {
            basepath: basepath.into(),
            env: Environment::new(),
        }
    }
}

impl LocalTemplater {
    /// Renders a template based on the provided `TemplateSelector` and the given `PipelineContext`.
    ///
    /// # Parameters
    ///
    /// * `context` - A mutable reference to the `PipelineContext` containing the event data and context.
    /// * `template` - A `TemplateSelector` specifying the template to be rendered.
    ///
    /// # Returns
    ///
    /// * `Result<RenderResponse, EngineError>` - On success, returns a `RenderResponse` containing the rendered template data.
    ///   On failure, returns an `EngineError` indicating the type of error that occurred during rendering.
    async fn render_template(
        &self,
        context: &mut PipelineContext,
        template: TemplateSelector,
    ) -> Result<RenderResponse, EngineError> {
        // Construct the path to the template descriptor file based on the channel and template name
        let template_dir = self.basepath.join(&context.channel);
        let descriptor_path = match template {
            TemplateSelector::ByName(name) => template_dir.join(name),
        };

        // Read the template descriptor file
        let descriptor = tokio::fs::read_to_string(descriptor_path)
            .await
            .map_err(|_| EngineError::TemplateRenderingError)?;

        // Parse the template descriptor JSON into a map
        let descriptor: Descriptor = toml::from_str(&descriptor).unwrap();

        // Initialize an empty map to store the rendered template data
        let mut data = Map::new();

        // Iterate over the parts in the template descriptor
        for (part, selector) in descriptor.template {
            // Deserialize the template source selector
            let selector: TemplateSourceSelector = selector.try_into().unwrap();

            // Determine the template source based on the selector
            let content = match selector {
                TemplateSourceSelector::Content(content) => content,
                TemplateSourceSelector::File(file) => {
                    // Read the template content from a file
                    tokio::fs::read_to_string(template_dir.join(file))
                        .await
                        .map_err(|_| EngineError::TemplateRenderingError)?
                }
            };

            // Render the template using the minijinja environment and the event context
            let rendered_tpl = self
                .env
                .render_str(&content, context.event_context.clone())
                .unwrap();

            // Insert the rendered template part into the data map
            data.insert(part, Value::String(rendered_tpl));
        }

        // Return the rendered template data
        Ok(RenderResponse(data))
    }
}

#[async_trait]
impl EnginePlugin for LocalTemplater {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::Load { templates } => {
                for template in templates {
                    let rendered_template = self.render_template(context, template).await?;
                    info!("{:?}", rendered_template);
                    context.messages.push(rendered_template);
                }

                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        vec!["templates.load".into()]
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum TemplateSelector {
    ByName(String),
}

/// Represents a step in the notification pipeline.
#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
enum Step {
    /// Loads templates for rendering.
    ///
    /// # Parameters
    ///
    /// * `templates` - A vector of `TemplateSelector` specifying the templates to be loaded.
    #[serde(rename = "templates.load")]
    Load { templates: Vec<TemplateSelector> },
}
