use crate::error::EngineError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RenderedTemplate {
    pub parts: HashMap<String, String>,
    pub extras: HashMap<String, String>,
}

impl RenderedTemplate {
    pub fn get(&self, name: &str) -> Result<&str, EngineError> {
        Ok(self
            .parts
            .get(name)
            .ok_or_else(|| EngineError::MissingTemplateParameter(name.to_string()))?)
    }
}
