use serde::{Deserialize, Serialize};

pub const TG_BODY: &str = "telegram.body";

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    #[serde(rename = "telegram.body")]
    pub body: String,
}
