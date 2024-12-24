use crate::credentials::GotifyCredentials;
use async_trait::async_trait;
use notifico_core::credentials::RawCredential;
use notifico_core::engine::{Message, PipelineContext};
use notifico_core::error::EngineError;
use notifico_core::recipient::RawContact;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

mod credentials;

#[derive(Serialize)]
struct Request {
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
        credential: RawCredential,
        _contact: RawContact,
        message: Message,
        _context: &mut PipelineContext,
    ) -> Result<(), EngineError> {
        let credential: GotifyCredentials = credential.try_into()?;
        let content: Content = message.content.try_into()?;

        let request = Request {
            title: content.title,
            message: content.body,
            priority: None,
            extras: None,
        };

        let url = format!("{}message?token={}", credential.base_url, credential.token);

        println!("Sending message to Gotify: {}", url);

        self.client
            .post(url)
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
