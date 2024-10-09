use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessagingProduct {
    Whatsapp,
}

#[derive(Serialize, Deserialize)]
pub struct Language {
    pub code: String,
}

impl From<&str> for Language {
    fn from(value: &str) -> Self {
        Self {
            code: value.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub messaging_product: MessagingProduct,
    pub to: String,
    pub language: Language,
    #[serde(flatten)]
    pub message: MessageType,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text { preview_url: bool, body: String },
}
