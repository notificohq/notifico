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

    fn into_inner(self) -> serde_json::Map<String, Value> {
        self.0
    }

    fn into_value(self) -> Value {
        Value::Object(self.into_inner())
    }

    pub fn convert_step<T>(self) -> Result<T, EngineError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let s = serde_json::to_string(&self.into_value()).map_err(EngineError::InvalidStep)?;
        serde_json::from_str(&s).map_err(EngineError::InvalidStep)
    }
}
