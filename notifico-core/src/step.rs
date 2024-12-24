use crate::error::EngineError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(transparent)]
pub struct SerializedStep(pub serde_json::Map<String, Value>);

impl SerializedStep {
    pub fn get_type(&self) -> &str {
        self.0["step"].as_str().expect("Step type must be a string")
    }

    pub fn convert_step<T>(&self) -> Result<T, EngineError>
    where
        T: for<'de> Deserialize<'de>,
    {
        T::deserialize(&self.0).map_err(EngineError::InvalidStep)
    }
}
