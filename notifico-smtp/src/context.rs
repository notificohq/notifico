use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginContext {
    #[serde(rename = "email.list_unsubscribe")]
    pub list_unsubscribe: Option<String>,
}
