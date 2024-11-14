use crate::error::EngineError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RenderedTemplate(pub HashMap<String, String>);

impl RenderedTemplate {
    pub fn get(&self, name: &str) -> Result<&str, EngineError> {
        Ok(self
            .0
            .get(name)
            .ok_or_else(|| EngineError::MissingTemplateParameter(name.to_string()))?)
    }
}
