mod step;

use crate::step::{Step, STEPS};
use async_trait::async_trait;
use notifico_core::credentials::{CredentialStorage, TypedCredential};
use notifico_core::engine::{EnginePlugin, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::recipient::TypedContact;
use notifico_core::step::SerializedStep;
use notifico_core::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use serde_json::json;
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
    client: reqwest::Client,
    credentials: Arc<dyn CredentialStorage>,
}

impl SlackPlugin {
    pub fn new(credentials: Arc<dyn CredentialStorage>) -> Self {
        SlackPlugin {
            client: reqwest::Client::new(),
            credentials,
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
                let Some(recipient) = context.recipient.clone() else {
                    return Err(EngineError::RecipientNotSet);
                };

                let credential: SlackCredentials = self
                    .credentials
                    .get_typed_credential(context.project_id, &credential)
                    .await?;

                let contact: SlackContact = recipient.get_primary_contact()?;

                for message in context.messages.iter().cloned() {
                    let content: SlackMessage = message.try_into()?;

                    let payload = json!({
                        "channel": contact.channel_id,
                        "text": content.text,
                    });

                    self.client
                        .post("https://slack.com/api/chat.postMessage")
                        .header("Authorization", format!("Bearer {}", credential.token))
                        .json(&payload)
                        .send()
                        .await
                        .unwrap();
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
