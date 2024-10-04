use crate::cloudapi::{Language, MessageType, MessagingProduct};
use crate::context::{Message, WA_BODY};
use crate::credentials::WhatsAppCredentials;
use crate::step::{Step, STEPS};
use async_trait::async_trait;
use notifico_core::credentials::{get_typed_credential, Credentials};
use notifico_core::engine::plugin::{EnginePlugin, StepOutput};
use notifico_core::engine::PipelineContext;
use notifico_core::error::EngineError;
use notifico_core::pipeline::SerializedStep;
use notifico_core::recipient::MobilePhoneContact;
use notifico_core::templater::{RenderResponse, Templater};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::sync::Arc;
use tracing::debug;

mod cloudapi;
mod context;
mod credentials;
mod step;

const CHANNEL_NAME: &str = "whatsapp";

pub struct WaBusinessPlugin {
    templater: Arc<dyn Templater>,
    credentials: Arc<dyn Credentials>,
    client: reqwest::Client,
}

impl WaBusinessPlugin {
    pub fn new(templater: Arc<dyn Templater>, credentials: Arc<dyn Credentials>) -> Self {
        Self {
            templater,
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
            Step::LoadTemplate { template_id } => {
                if context.recipient.is_none() {
                    return Err(EngineError::RecipientNotSet);
                };

                let rendered_template = self
                    .templater
                    .render(CHANNEL_NAME, template_id, context.event_context.0.clone())
                    .await?;
                let rendered_template: WhatsAppContent = rendered_template.try_into().unwrap();

                context
                    .plugin_contexts
                    .insert(WA_BODY.into(), Value::String(rendered_template.body));
            }
            Step::Send { credential } => {
                let Some(recipient) = context.recipient.clone() else {
                    return Err(EngineError::RecipientNotSet);
                };

                let message: Message =
                    serde_json::from_value(context.plugin_contexts.clone().into()).unwrap();

                let contact: MobilePhoneContact = recipient.get_primary_contact()?;

                // Send
                let credential: WhatsAppCredentials = get_typed_credential(
                    self.credentials.as_ref(),
                    context.project_id,
                    &credential,
                )?;

                let wamessage = cloudapi::Message {
                    messaging_product: MessagingProduct::Whatsapp,
                    to: contact.number,
                    language: Language {
                        code: "en_US".into(),
                    },
                    message: MessageType::Text {
                        preview_url: false,
                        body: message.body,
                    },
                };

                let url = format!(
                    "https://graph.facebook.com/v20.0/{}/messages",
                    credential.phone_id
                );
                let result = self
                    .client
                    .post(url)
                    .header("Authorization", format!("Bearer {}", credential.token))
                    .json(&wamessage)
                    .send()
                    .await
                    .unwrap();
                debug!("Response: {:?}", result);
            }
        }

        Ok(StepOutput::None)
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct WhatsAppContent {
    pub body: String,
}

impl TryFrom<RenderResponse> for WhatsAppContent {
    type Error = ();

    fn try_from(value: RenderResponse) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0)).map_err(|_| ())
    }
}
