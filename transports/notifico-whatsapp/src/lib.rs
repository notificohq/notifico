use crate::cloudapi::{MessageType, MessagingProduct};
use crate::credentials::WhatsAppCredentials;
use crate::step::{Step, STEPS};
use async_trait::async_trait;
use notifico_core::contact::MobilePhoneContact;
use notifico_core::recorder::Recorder;
use notifico_core::step::SerializedStep;
use notifico_core::transport::Transport;
use notifico_core::{
    credentials::CredentialStorage,
    engine::PipelineContext,
    engine::{EnginePlugin, StepOutput},
    error::EngineError,
    templater::RenderedTemplate,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::sync::Arc;

mod cloudapi;
mod credentials;
mod step;

pub struct WaBusinessPlugin {
    credentials: Arc<dyn CredentialStorage>,
    recorder: Arc<dyn Recorder>,
    client: reqwest::Client,
}

impl WaBusinessPlugin {
    pub fn new(credentials: Arc<dyn CredentialStorage>, recorder: Arc<dyn Recorder>) -> Self {
        Self {
            credentials,
            recorder,
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
                let contacts: Vec<MobilePhoneContact> = context.get_recipient()?.get_contacts();

                // Send
                let credential: WhatsAppCredentials = self
                    .credentials
                    .resolve(context.project_id, credential)
                    .await?;

                let url = format!(
                    "https://graph.facebook.com/v20.0/{}/messages",
                    credential.phone_id
                );

                for contact in contacts {
                    for message in context.messages.iter().cloned() {
                        let wa_message: WhatsAppContent = message.content.try_into().unwrap();

                        let wamessage = cloudapi::Message {
                            messaging_product: MessagingProduct::Whatsapp,
                            to: contact.number.clone(),
                            language: "en_US".into(),
                            message: MessageType::Text {
                                preview_url: false,
                                body: wa_message.body,
                            },
                        };

                        let result = self
                            .client
                            .post(url.clone())
                            .header("Authorization", format!("Bearer {}", credential.token))
                            .json(&wamessage)
                            .send()
                            .await;
                        match result {
                            Ok(_) => self.recorder.record_message_sent(
                                context.event_id,
                                context.notification_id,
                                message.id,
                            ),
                            Err(e) => self.recorder.record_message_failed(
                                context.event_id,
                                context.notification_id,
                                message.id,
                                &e.to_string(),
                            ),
                        }
                    }
                }
            }
        }

        Ok(StepOutput::Continue)
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}

impl Transport for WaBusinessPlugin {
    fn name(&self) -> Cow<'static, str> {
        "waba".into()
    }

    fn send_step(&self) -> Cow<'static, str> {
        "waba.send".into()
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
