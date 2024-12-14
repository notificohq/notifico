use async_trait::async_trait;
use notifico_core::contact::{RawContact, TypedContact};
use notifico_core::credentials::{RawCredential, TypedCredential};
use notifico_core::engine::{Message, PipelineContext};
use notifico_core::error::EngineError;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct PushoverCredentials {
    token: String,
}

impl TryFrom<RawCredential> for PushoverCredentials {
    type Error = EngineError;

    fn try_from(value: RawCredential) -> Result<Self, Self::Error> {
        Ok(Self { token: value.value })
    }
}

impl TypedCredential for PushoverCredentials {
    const TRANSPORT_NAME: &'static str = "pushover";
}

#[derive(Serialize)]
struct PushoverMessageRequest {
    token: String,
    user: String,
    message: String,

    attachment_base64: Option<String>,
    attachment_type: Option<String>,

    device: Option<String>,
    html: Option<u8>,
    priority: Option<i8>,
    sound: Option<String>,
    timestamp: Option<u64>,
    title: Option<String>,
    ttl: Option<u64>,
    url: Option<Url>,
    url_title: Option<String>,
}

pub struct PushoverTransport {
    client: reqwest::Client,
}

impl PushoverTransport {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SimpleTransport for PushoverTransport {
    async fn send_message(
        &self,
        credential: RawCredential,
        contact: RawContact,
        message: Message,
        _context: &mut PipelineContext,
    ) -> Result<(), EngineError> {
        let credential: PushoverCredentials = credential.try_into()?;
        let contact: PushoverContact = contact.try_into()?;
        let message: PushoverMessage = message.content.try_into()?;

        let request = PushoverMessageRequest {
            token: credential.token.clone(),
            user: contact.user.clone(),
            message: message.body,
            attachment_base64: None,
            attachment_type: None,
            device: None,
            html: Some(1),
            priority: None,
            sound: None,
            timestamp: None,
            title: Some(message.title),
            ttl: None,
            url: None,
            url_title: None,
        };

        self.client
            .post("https://api.pushover.net/1/messages.json")
            .body(serde_urlencoded::to_string(request).unwrap_or_default())
            .send()
            .await
            .map_err(|e| EngineError::InternalError(e.into()))?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "pushover"
    }

    fn supports_contact(&self, r#type: &str) -> bool {
        r#type == "pushover"
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct PushoverContact {
    user: String,
}

impl TryFrom<RawContact> for PushoverContact {
    type Error = EngineError;

    fn try_from(value: RawContact) -> Result<Self, Self::Error> {
        Ok(Self { user: value.value })
    }
}

impl TypedContact for PushoverContact {
    const CONTACT_TYPE: &'static str = "pushover";
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PushoverMessage {
    pub body: String,
    pub title: String,
}

impl TryFrom<RenderedTemplate> for PushoverMessage {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        Ok(Self {
            body: value.get("body")?.to_string(),
            title: value.get("title")?.to_string(),
        })
    }
}
