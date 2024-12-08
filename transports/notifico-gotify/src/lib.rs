use crate::credentials::GotifyCredentials;
use async_trait::async_trait;
use notifico_core::contact::Contact;
use notifico_core::credentials::Credential;
use notifico_core::error::EngineError;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

mod credentials;

#[derive(Serialize)]
struct GotifyRequest {
    title: Option<String>,
    message: String,
    priority: Option<i8>,
    extras: Option<Map<String, Value>>,
}

#[derive(Default)]
pub struct GotifyTransport {
    client: reqwest::Client,
}

impl GotifyTransport {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SimpleTransport for GotifyTransport {
    async fn send_message(
        &self,
        credential: Credential,
        _contact: Contact,
        message: RenderedTemplate,
    ) -> Result<(), EngineError> {
        let credential: GotifyCredentials = credential.try_into()?;
        let content: GotifyContent = message.try_into()?;

        let request = GotifyRequest {
            title: content.title,
            message: content.body,
            priority: None,
            extras: None,
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
        "gotify"
    }

    fn has_contacts(&self) -> bool {
        false
    }

    fn supports_contact(&self, _type: &str) -> bool {
        false
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct GotifyContent {
    title: Option<String>,
    body: String,
}

impl TryFrom<RenderedTemplate> for GotifyContent {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0))
            .map_err(|e| EngineError::InvalidRenderedTemplateFormat(e.into()))
    }
}
