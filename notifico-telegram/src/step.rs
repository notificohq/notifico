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
pub enum TelegramStep {
    #[serde(rename = "telegram.load_template")]
    LoadTemplate { template_id: Uuid },
    // #[serde(rename = "telegram.set_recipients")]
    // SetRecipients { telegram_id: Vec<i64> },
    #[serde(rename = "telegram.send")]
    Send(CredentialSelector),
}

impl TryFrom<SerializedStep> for TelegramStep {
    type Error = ();

    fn try_from(value: SerializedStep) -> Result<Self, Self::Error> {
        let s = serde_json::to_string(&value.into_value()).unwrap();

        Ok(serde_json::from_str(&s).unwrap())
    }
}
