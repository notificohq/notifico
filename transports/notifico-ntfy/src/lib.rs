use async_trait::async_trait;
use notifico_core::contact::Contact;
use notifico_core::credentials::{Credential, TypedCredential};
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
struct NtfyRequest {
    topic: String,
    message: Option<String>,
    title: Option<String>,
}

#[async_trait]
impl SimpleTransport for NtfyTransport {
    async fn send_message(
        &self,
        credential: Credential,
        contact: Contact,
        message: RenderedTemplate,
    ) -> Result<(), EngineError> {
        let credential: Credentials = credential.try_into()?;
        let content: NtfyContent = message.try_into()?;
        let contact: NtfyContact = contact.try_into()?;

        let request = NtfyRequest {
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
pub struct Credentials {
    pub url: Url,
}

impl TryFrom<Credential> for Credentials {
    type Error = EngineError;

    fn try_from(value: Credential) -> Result<Self, Self::Error> {
        let url = Url::parse(&value.value).map_err(|_| EngineError::InvalidCredentialFormat)?;

        Ok(Self { url })
    }
}

impl TypedCredential for Credentials {
    const TRANSPORT_NAME: &'static str = "ntfy";
}

#[derive(Deserialize, Clone)]
struct NtfyContent {
    title: Option<String>,
    body: String,
}

impl TryFrom<RenderedTemplate> for NtfyContent {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0))
            .map_err(|e| EngineError::InvalidRenderedTemplateFormat(e.into()))
    }
}

struct NtfyContact {
    topic: String,
}

impl TryFrom<Contact> for NtfyContact {
    type Error = EngineError;

    fn try_from(value: Contact) -> Result<Self, Self::Error> {
        Ok(Self { topic: value.value })
    }
}
