use async_trait::async_trait;
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use notifico_attachment::AttachmentPlugin;
use notifico_core::contact::RawContact;
use notifico_core::credentials::{RawCredential, TypedCredential};
use notifico_core::engine::Message;
use notifico_core::error::EngineError;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use url::Url;

pub struct NtfyTransport {
    client: reqwest::Client,
    attachments: Arc<AttachmentPlugin>,
}

impl NtfyTransport {
    pub fn new(client: reqwest::Client, attachments: Arc<AttachmentPlugin>) -> Self {
        Self {
            client,
            attachments,
        }
    }
}

#[derive(Serialize)]
struct Request {
    topic: String,
    message: Option<String>,
    title: Option<String>,
}

#[async_trait]
impl SimpleTransport for NtfyTransport {
    async fn send_message(
        &self,
        credential: RawCredential,
        contact: RawContact,
        message: Message,
    ) -> Result<(), EngineError> {
        let credential: Credential = credential.try_into()?;
        let content: Content = message.content.try_into()?;
        let contact: Contact = contact.try_into()?;

        let request = Request {
            topic: contact.topic.clone(),
            message: Some(content.body),
            title: content.title,
        };

        self.client
            .post(credential.url.clone())
            .json(&request)
            .send()
            .await
            .map_err(|e| EngineError::InternalError(e.into()))?;

        for attachment in &message.attachments {
            let mut file = self.attachments.get_attachment(attachment).await?;

            if !file.file_name.is_ascii() {
                file.file_name = format!(
                    "=?UTF-8?B?{}?=",
                    BASE64_STANDARD.encode(file.file_name.as_bytes())
                );
            }

            let mut filebody = vec![];
            file.file.read_to_end(&mut filebody).await?;

            self.client
                .post(credential.url.clone().join(&contact.topic).unwrap())
                .header("Filename", file.file_name)
                .body(filebody)
                .send()
                .await
                .map_err(|e| EngineError::InternalError(e.into()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ntfy"
    }

    fn supports_contact(&self, r#type: &str) -> bool {
        r#type == "ntfy"
    }
}

#[derive(Serialize, Deserialize)]
pub struct Credential {
    pub url: Url,
}

impl TryFrom<RawCredential> for Credential {
    type Error = EngineError;

    fn try_from(value: RawCredential) -> Result<Self, Self::Error> {
        let url = Url::parse(&value.value).map_err(|_| EngineError::InvalidCredentialFormat)?;

        Ok(Self { url })
    }
}

impl TypedCredential for Credential {
    const TRANSPORT_NAME: &'static str = "ntfy";
}

#[derive(Deserialize, Clone)]
struct Content {
    title: Option<String>,
    body: String,
}

impl TryFrom<RenderedTemplate> for Content {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0))
            .map_err(|e| EngineError::InvalidRenderedTemplateFormat(e.into()))
    }
}

struct Contact {
    topic: String,
}

impl TryFrom<RawContact> for Contact {
    type Error = EngineError;

    fn try_from(value: RawContact) -> Result<Self, Self::Error> {
        Ok(Self { topic: value.value })
    }
}
