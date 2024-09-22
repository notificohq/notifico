use crate::error::EngineError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Credential {
    pub r#type: String,
    pub name: String,
    pub value: Value,
}

pub trait Credentials: Send + Sync {
    fn get_credential(&self, project: Uuid, r#type: &str, name: &str)
        -> Result<Value, EngineError>;
}
