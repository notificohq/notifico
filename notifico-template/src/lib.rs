pub mod entity;
pub mod error;
pub mod source;

use async_trait::async_trait;
use error::TemplaterError;
use minijinja::Environment;
use notifico_core::engine::{EnginePlugin, Message, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::step::SerializedStep;
use notifico_core::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use source::TemplateSource;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct PreRenderedTemplate {
    pub parts: HashMap<String, String>,
    // attachments: Vec<String>,
    // extras: HashMap<String, String>,
}

pub struct Templater {
    source: Arc<dyn TemplateSource>,
    env: Environment<'static>,
}

impl Templater {
    pub fn new(source: Arc<dyn TemplateSource>) -> Self {
        Self {
            source,
            env: Environment::new(),
        }
    }
}

impl Templater {
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
        template: PreRenderedTemplate,
        context: &Map<String, Value>,
    ) -> Result<RenderedTemplate, TemplaterError> {
        let mut data = HashMap::new();
        for (part_name, part_content) in template.parts {
            // Render the template using the minijinja environment and the event context
            let rendered_tpl = self.env.render_str(&part_content, context)?;

            // Insert the rendered template part into the data map
            data.insert(part_name, rendered_tpl);
        }

        // Return the rendered template data
        Ok(RenderedTemplate(data))
    }
}

#[async_trait]
impl EnginePlugin for Templater {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.convert_step()?;

        match step {
            Step::Load { templates } => {
                for template in templates {
                    let message_id = Uuid::now_v7();
                    debug!("Assigned Message ID: {:?}", message_id);

                    // Template
                    let template = match template {
                        TemplateSelector::Inline { inline } => inline,
                        sel => self.source.get_template(context.project_id, sel).await?,
                    };

                    // Context
                    let mut template_context = context.event_context.0.clone();

                    let mut enrich_context = Map::new();

                    enrich_context.insert(
                        "message_id".to_owned(),
                        Value::String(message_id.to_string()),
                    );
                    enrich_context.insert(
                        "notification_id".to_owned(),
                        Value::String(context.notification_id.to_string()),
                    );

                    template_context.insert("_".to_owned(), Value::Object(enrich_context));

                    // Render
                    let rendered_template =
                        self.render_template(template, &template_context).await?;
                    context.messages.push(Message {
                        id: message_id,
                        content: rendered_template,
                        attachments: vec![],
                    });
                }

                Ok(StepOutput::Continue)
            }
            Step::LoadContext => {
                let message_id = Uuid::now_v7();
                debug!("Assigned Message ID: {:?}", message_id);

                // Stringify parts
                let mut rendered_parts: HashMap<String, String> = HashMap::new();
                for (part, value) in context.event_context.0.iter() {
                    let value = match value.as_str() {
                        None => {
                            debug!("Value for part '{part}' is not a string");
                            value.to_string()
                        }
                        Some(value) => value.to_string(),
                    };
                    rendered_parts.insert(part.clone(), value);
                }

                // Set context as a rendered template
                let rendered_template: RenderedTemplate = RenderedTemplate(rendered_parts);
                context.messages.push(Message {
                    id: message_id,
                    content: rendered_template,
                    attachments: vec![],
                });

                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        vec!["templates.load".into(), "templates.load-context".into()]
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TemplateSelector {
    Name(String),
    Inline { inline: PreRenderedTemplate },
    File { file: String },
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
    /// Loads context as already rendered template, effectively bypassing templating process.
    /// Useful for passing pre-rendered data into the notification pipeline.
    #[serde(rename = "templates.load-context")]
    LoadContext,
}
