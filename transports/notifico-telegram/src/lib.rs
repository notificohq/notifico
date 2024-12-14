use async_trait::async_trait;
use contact::TelegramContact;
use notifico_attachment::AttachmentPlugin;
use notifico_core::contact::RawContact;
use notifico_core::credentials::RawCredential;
use notifico_core::engine::Message;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::{
    credentials::TypedCredential, error::EngineError, templater::RenderedTemplate,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

const API_URL: &str = "https://api.telegram.org/bot";

mod contact;

#[derive(Debug, Serialize, Deserialize)]
struct TelegramBotCredentials {
    token: String,
}

impl TryFrom<RawCredential> for TelegramBotCredentials {
    type Error = EngineError;

    fn try_from(value: RawCredential) -> Result<Self, Self::Error> {
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
    attachments: Arc<AttachmentPlugin>,
}

impl TelegramTransport {
    pub fn new(client: reqwest::Client, attachments: Arc<AttachmentPlugin>) -> Self {
        Self {
            client,
            attachments,
        }
    }
}

#[async_trait]
impl SimpleTransport for TelegramTransport {
    async fn send_message(
        &self,
        credential: RawCredential,
        contact: RawContact,
        message: Message,
    ) -> Result<(), EngineError> {
        let credential: TelegramBotCredentials = credential.try_into()?;
        let contact: TelegramContact = contact.try_into()?;
        let content: TelegramContent = message.content.try_into()?;

        if message.attachments.is_empty() {
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
        } else {
            // Consider sending attachments using sendMediaGroup
            let url = format!("{}{}/sendDocument", API_URL, credential.token);

            let attach = self
                .attachments
                .get_attachment(&message.attachments[0])
                .await?;

            let form = reqwest::multipart::Form::new()
                .text("chat_id", contact.chat_id.to_string())
                .text("caption", content.body)
                .part(
                    "document",
                    reqwest::multipart::Part::stream(attach.file)
                        .file_name(attach.file_name.clone()),
                );
            self.client
                .post(url)
                .multipart(form)
                .send()
                .await
                .map_err(|e| EngineError::InternalError(e.into()))?;
        }
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
