use serde::{Deserialize, Serialize};

pub const WA_BODY: &str = "wa.body";

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    #[serde(rename = "wa.body")]
    pub body: String,
}
