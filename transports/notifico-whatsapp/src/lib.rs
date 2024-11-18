use crate::cloudapi::{MessageType, MessagingProduct};
use crate::credentials::WhatsAppCredentials;
use crate::step::{Step, STEPS};
use async_trait::async_trait;
use notifico_core::step::SerializedStep;
use notifico_core::{
    credentials::CredentialStorage,
    engine::PipelineContext,
    engine::{EnginePlugin, StepOutput},
    error::EngineError,
    recipient::MobilePhoneContact,
    templater::RenderedTemplate,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::sync::Arc;
use tracing::debug;

mod cloudapi;
mod credentials;
mod step;

pub struct WaBusinessPlugin {
    credentials: Arc<dyn CredentialStorage>,
    client: reqwest::Client,
}

impl WaBusinessPlugin {
    pub fn new(credentials: Arc<dyn CredentialStorage>) -> Self {
        Self {
            credentials,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl EnginePlugin for WaBusinessPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::Send { credential } => {
                let contact: MobilePhoneContact = context.get_contact()?;

                // Send
                let credential: WhatsAppCredentials = self
                    .credentials
                    .get_typed_credential(context.project_id, &credential)
                    .await?;

                let url = format!(
                    "https://graph.facebook.com/v20.0/{}/messages",
                    credential.phone_id
                );

                for message in context.messages.iter().cloned() {
                    let message: WhatsAppContent = message.try_into().unwrap();

                    let wamessage = cloudapi::Message {
                        messaging_product: MessagingProduct::Whatsapp,
                        to: contact.number.clone(),
                        language: "en_US".into(),
                        message: MessageType::Text {
                            preview_url: false,
                            body: message.body,
                        },
                    };

                    let result = self
                        .client
                        .post(url.clone())
                        .header("Authorization", format!("Bearer {}", credential.token))
                        .json(&wamessage)
                        .send()
                        .await
                        .unwrap();
                    debug!("Response: {:?}", result);
                }
            }
        }

        Ok(StepOutput::Continue)
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct WhatsAppContent {
    pub body: String,
}

impl TryFrom<RenderedTemplate> for WhatsAppContent {
    type Error = ();

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0)).map_err(|_| ())
    }
}
