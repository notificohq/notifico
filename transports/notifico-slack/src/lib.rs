mod slackapi;
mod step;

use crate::step::{Step, STEPS};
use async_trait::async_trait;
use notifico_core::credentials::{CredentialStorage, TypedCredential};
use notifico_core::engine::{EnginePlugin, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::recipient::TypedContact;
use notifico_core::recorder::Recorder;
use notifico_core::step::SerializedStep;
use notifico_core::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct SlackCredentials {
    token: String,
}

impl TypedCredential for SlackCredentials {
    const CREDENTIAL_TYPE: &'static str = "slack";
}

pub struct SlackPlugin {
    client: slackapi::SlackApi,
    credentials: Arc<dyn CredentialStorage>,
    recorder: Arc<dyn Recorder>,
}

impl SlackPlugin {
    pub fn new(credentials: Arc<dyn CredentialStorage>, recorder: Arc<dyn Recorder>) -> Self {
        SlackPlugin {
            client: slackapi::SlackApi::new(),
            credentials,
            recorder,
        }
    }
}

#[async_trait]
impl EnginePlugin for SlackPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::Send { credential } => {
                let credential: SlackCredentials = self
                    .credentials
                    .get_typed_credential(context.project_id, &credential)
                    .await?;

                let contact: SlackContact = context.get_contact()?;

                for message in context.messages.iter().cloned() {
                    let content: SlackMessage = message.content.try_into()?;
                    let slack_message = slackapi::SlackMessage::Text {
                        channel: contact.channel_id.clone(),
                        text: content.text,
                    };

                    let result = self
                        .client
                        .chat_post_message(&credential.token, slack_message)
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
                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct SlackContact {
    channel_id: String,
}

impl TypedContact for SlackContact {
    const CONTACT_TYPE: &'static str = "slack";
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SlackMessage {
    pub text: String,
}

impl TryFrom<RenderedTemplate> for SlackMessage {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        Ok(Self {
            text: value.get("text")?.to_string(),
        })
    }
}
