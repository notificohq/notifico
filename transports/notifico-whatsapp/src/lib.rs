use crate::cloudapi::{MessageType, MessagingProduct};
use crate::credentials::WhatsAppCredentials;
use async_trait::async_trait;
use notifico_core::contact::{Contact, MobilePhoneContact};
use notifico_core::credentials::Credential;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::{error::EngineError, templater::RenderedTemplate};
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod cloudapi;
mod credentials;

pub struct WabaTransport {
    client: reqwest::Client,
}

impl WabaTransport {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl SimpleTransport for WabaTransport {
    async fn send_message(
        &self,
        credential: Credential,
        contact: Contact,
        message: RenderedTemplate,
    ) -> Result<(), EngineError> {
        let credential: WhatsAppCredentials = credential.try_into()?;
        let contact: MobilePhoneContact = contact.try_into()?;
        let message: WhatsAppContent = message.try_into()?;

        let url = format!(
            "https://graph.facebook.com/v20.0/{}/messages",
            credential.phone_id
        );

        let request = cloudapi::Message {
            messaging_product: MessagingProduct::Whatsapp,
            to: contact.number.clone(),
            language: "en_US".into(),
            message: MessageType::Text {
                preview_url: false,
                body: message.body,
            },
        };

        self.client
            .post(url)
            .header("Authorization", format!("Bearer {}", credential.token))
            .json(&request)
            .send()
            .await
            .map_err(|e| EngineError::InternalError(e.into()))?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "waba"
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct WhatsAppContent {
    pub body: String,
}

impl TryFrom<RenderedTemplate> for WhatsAppContent {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0))
            .map_err(|e| EngineError::InternalError(e.into()))
    }
}
