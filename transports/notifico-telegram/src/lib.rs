use async_trait::async_trait;
use contact::TelegramContact;
use notifico_core::contact::Contact;
use notifico_core::credentials::Credential;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::{
    credentials::TypedCredential, error::EngineError, templater::RenderedTemplate,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const API_URL: &str = "https://api.telegram.org/bot";

mod contact;

#[derive(Debug, Serialize, Deserialize)]
struct TelegramBotCredentials {
    token: String,
}

impl TryFrom<Credential> for TelegramBotCredentials {
    type Error = EngineError;

    fn try_from(value: Credential) -> Result<Self, Self::Error> {
        Ok(Self { token: value.value })
    }
}

#[derive(Serialize)]
struct SendMessageRequest {
    chat_id: i64,
    text: String,
}

impl TypedCredential for TelegramBotCredentials {
    const TRANSPORT_NAME: &'static str = "telegram";
}

pub struct TelegramTransport {
    client: reqwest::Client,
}

impl TelegramTransport {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SimpleTransport for TelegramTransport {
    async fn send_message(
        &self,
        credential: Credential,
        contact: Contact,
        message: RenderedTemplate,
    ) -> Result<(), EngineError> {
        let credential: TelegramBotCredentials = credential.try_into()?;
        let contact: TelegramContact = contact.try_into()?;
        let content: TelegramContent = message.try_into()?;

        let request = SendMessageRequest {
            chat_id: contact.chat_id,
            text: content.body,
        };

        let url = format!("{}{}/sendMessage", API_URL, credential.token);

        self.client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|e| EngineError::InternalError(e.into()))?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "telegram"
    }

    fn supports_contact(&self, r#type: &str) -> bool {
        r#type == "telegram"
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct TelegramContent {
    pub body: String,
}

impl TryFrom<RenderedTemplate> for TelegramContent {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0))
            .map_err(|e| EngineError::InvalidRenderedTemplateFormat(e.into()))
    }
}
