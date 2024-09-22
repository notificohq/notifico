use crate::error::EngineError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Pipeline {
    pub events: Vec<String>,
    pub steps: Vec<SerializedStep>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(transparent)]
pub struct SerializedStep(pub serde_json::Map<String, Value>);

impl SerializedStep {
    pub fn get_type(&self) -> &str {
        self.0["step"].as_str().expect("Step type must be a string")
    }

    pub fn get_namespace(&self) -> &str {
        self.get_type().split(".").next().unwrap_or_default()
    }

    fn into_inner(self) -> serde_json::Map<String, Value> {
        self.0
    }

    pub fn into_value(self) -> Value {
        Value::Object(self.into_inner())
    }
}

pub trait PipelineStorage {
    fn get_pipelines(&self, project: Uuid, event_name: &str) -> Result<Vec<Pipeline>, EngineError>;
}
