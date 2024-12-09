use async_trait::async_trait;
use notifico_core::contact::RawContact;
use notifico_core::credentials::{RawCredential, TypedCredential};
use notifico_core::error::EngineError;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

#[derive(Default)]
pub struct NtfyTransport {
    client: reqwest::Client,
}

impl NtfyTransport {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
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
        message: RenderedTemplate,
    ) -> Result<(), EngineError> {
        let credential: Credential = credential.try_into()?;
        let content: Content = message.try_into()?;
        let contact: Contact = contact.try_into()?;

        let request = Request {
            topic: contact.topic,
            message: Some(content.body),
            title: content.title,
        };

        self.client
            .post(credential.url.clone())
            .json(&request)
            .send()
            .await
            .map_err(|e| EngineError::InternalError(e.into()))?;
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
