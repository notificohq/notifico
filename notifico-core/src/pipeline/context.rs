use crate::error::EngineError;
use crate::pipeline::Pipeline;
use crate::recipient::Recipient;
use crate::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use url::Url;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Default, Clone, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
/// Event context contains all variables, that will be passed to templating engine.
pub struct EventContext(pub Map<String, Value>);

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AttachmentMetadata {
    pub url: Url,
    pub file_name: Option<String>,
    pub extras: HashMap<String, String>,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub content: RenderedTemplate,
    pub attachments: Vec<AttachmentMetadata>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContext {
    pub pipeline: Pipeline,

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

    pub fn fork(&self) -> Self {
        let pipeline = Pipeline {
            project_id: self.project_id,
            steps: self.pipeline.steps[1..].to_vec(),
        };
        Self {
            pipeline,
            project_id: self.project_id,
            event_id: self.event_id,
            notification_id: Uuid::now_v7(),
            recipient: self.recipient.clone(),
            event_name: self.event_name.clone(),
            event_context: self.event_context.clone(),
            plugin_contexts: self.plugin_contexts.clone(),
            messages: self.messages.clone(),
        }
    }
}
