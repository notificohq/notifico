use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Pipeline {
    pub channel: String,
    pub steps: Vec<SerializedStep>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct SerializedStep(pub serde_json::Map<String, Value>);

impl SerializedStep {
    pub fn get_type(&self) -> &str {
        &self.0["type"].as_str().expect("Step type must be a string")
    }
}
