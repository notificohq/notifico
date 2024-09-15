use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Step {
    #[serde(rename = "telegram.load_template")]
    TgLoadTemplate { template_id: Uuid },
    #[serde(rename = "telegram.send")]
    TgSend { bot_token: String },

    #[serde(rename = "email.load_template")]
    EmailLoadTemplate { template_id: Uuid },
    #[serde(rename = "email.premailer")]
    EmailPremailer,
    #[serde(rename = "email.send")]
    EmailSend { bot_token: String },

    #[serde(rename = "slack.load_template")]
    SlackLoadTemplate { template_id: Uuid },
    #[serde(rename = "slack.send")]
    SlackSend { bot_token: String },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TelegramStep {
    #[serde(rename = "telegram.load_template")]
    TgLoadTemplate { template_id: Uuid },
    #[serde(rename = "telegram.send")]
    TgSend { bot_token: String },
}
