use notifico_core::pipeline::SerializedStep;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum CredentialSelector {
    BotName { bot_name: String },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "telegram.load_template")]
    LoadTemplate { template_id: Uuid },
    // #[serde(rename = "telegram.set_recipients")]
    // SetRecipients { telegram_id: Vec<i64> },
    #[serde(rename = "telegram.send")]
    Send(CredentialSelector),
}

pub(crate) const STEPS: &'static [&'static str] = &["telegram.load_template", "telegram.send"];
