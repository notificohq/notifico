use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Credential {
    pub r#type: String,
    pub name: String,
    pub value: Value,
}

pub trait Credentials: Send + Sync {
    fn get_credential(&self, r#type: &str, name: &str) -> Option<Value>;
}
