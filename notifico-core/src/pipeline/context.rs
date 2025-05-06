use crate::pipeline::Pipeline;
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

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PipelineContext {
    pub pipeline: Pipeline,

    pub project_id: Uuid,
    pub event_id: Uuid,
    pub notification_id: Uuid,

    pub event_name: String,
    pub event_context: EventContext,
    pub plugin_contexts: Map<String, Value>,
}
