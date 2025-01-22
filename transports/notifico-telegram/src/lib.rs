use async_trait::async_trait;
use contact::TelegramContact;
use notifico_attachment::AttachmentPlugin;
use notifico_core::credentials::RawCredential;
use notifico_core::pipeline::context::{Message, PipelineContext};
use notifico_core::recipient::RawContact;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::{
    credentials::TypedCredential, error::EngineError, templater::RenderedTemplate,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
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

struct AttachmentStrategy {
    method: &'static str,
    key: &'static str,
}

impl AttachmentStrategy {
    fn get_strategy(telegram_type: &str) -> Result<AttachmentStrategy, EngineError> {
        match telegram_type {
            "document" => Ok(AttachmentStrategy {
                method: "sendDocument",
                key: "document",
            }),
            "photo" => Ok(AttachmentStrategy {
                method: "sendPhoto",
                key: "photo",
            }),
            "video" => Ok(AttachmentStrategy {
                method: "sendVideo",
                key: "video",
            }),
            "audio" => Ok(AttachmentStrategy {
                method: "sendAudio",
                key: "audio",
            }),
            "voice" => Ok(AttachmentStrategy {
                method: "sendVoice",
                key: "voice",
            }),
            _ => Err(EngineError::InvalidConfiguration(
                "Invalid attachment type".into(),
            )),
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
        _context: &mut PipelineContext,
    ) -> Result<(), EngineError> {
        let credential: TelegramBotCredentials = credential.try_into()?;
        let contact: TelegramContact = contact.try_into()?;
        let content: TelegramContent = message.content.try_into()?;

        let request = SendMessageRequest {
            chat_id: contact.chat_id,
            text: content.body.clone(),
        };

        let url = format!("{}{}/sendMessage", API_URL, credential.token);

        self.client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|e| EngineError::InternalError(e.into()))?;

        // Send attachments as separate messages
        for attachment in &message.attachments {
            // Consider sending attachments using sendMediaGroup
            let attach = self.attachments.get_attachment(attachment).await?;

            let attachment_type = attach
                .extras
                .get("telegram.type")
                .map(|s| s.as_str())
                .unwrap_or("document");

            let strategy = AttachmentStrategy::get_strategy(attachment_type)?;

            let url = format!("{}{}/{}", API_URL, credential.token, strategy.method);

            let form = reqwest::multipart::Form::new()
                .text("chat_id", contact.chat_id.to_string())
                .part(
                    strategy.key,
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

    fn supported_channels(&self) -> Vec<Cow<'static, str>> {
        vec!["telegram".into()]
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct TelegramContent {
    pub body: String,
}

impl TryFrom<RenderedTemplate> for TelegramContent {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.parts))
            .map_err(|e| EngineError::InvalidRenderedTemplateFormat(e.into()))
    }
}
