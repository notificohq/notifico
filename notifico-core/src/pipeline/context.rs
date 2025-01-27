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
    pub step_number: usize,

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
}
