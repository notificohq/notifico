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
use teloxide::prelude::Requester;
use teloxide::Bot;

mod contact;

#[derive(Debug, Serialize, Deserialize)]
struct TelegramBotCredentials {
    token: String,
}

impl TryFrom<Credential> for TelegramBotCredentials {
    type Error = EngineError;

    fn try_from(value: Credential) -> Result<Self, Self::Error> {
        if value.transport() != Self::TRANSPORT_NAME {
            return Err(EngineError::InvalidCredentialFormat)?;
        }

        match value {
            Credential::Long { value, .. } => {
                Ok(serde_json::from_value(value)
                    .map_err(|_| EngineError::InvalidCredentialFormat)?)
            }
            Credential::Short(url) => Ok(Self {
                token: url
                    .strip_prefix("telegram://")
                    .unwrap_or_default()
                    .to_owned(),
            }),
        }
    }
}

impl TypedCredential for TelegramBotCredentials {
    const TRANSPORT_NAME: &'static str = "telegram";
}

pub struct TelegramTransport {}

impl TelegramTransport {
    pub fn new() -> Self {
        Self {}
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
        let bot = Bot::new(credential.token);
        let contact: TelegramContact = contact.try_into()?;
        let content: TelegramContent = message.try_into()?;

        // Send
        bot.send_message(contact.clone().into_recipient(), content.body)
            .await
            .map_err(|e| EngineError::InternalError(e.into()))?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "telegram"
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
