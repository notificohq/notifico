use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RenderResponse(pub Map<String, Value>);
