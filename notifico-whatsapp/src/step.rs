use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "whatsapp.load_template")]
    LoadTemplate { template_id: Uuid },
    // #[serde(rename = "telegram.set_recipients")]
    // SetRecipients { telegram_id: Vec<i64> },
    #[serde(rename = "whatsapp.send")]
    Send { credential: String },
}

pub const STEPS: &[&str] = &["whatsapp.load_template", "whatsapp.send"];
