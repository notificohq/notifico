mod step;

use crate::step::{Step, STEPS};
use async_trait::async_trait;
use notifico_core::contact::{Contact, TypedContact};
use notifico_core::credentials::{Credential, CredentialStorage, TypedCredential};
use notifico_core::engine::{EnginePlugin, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::recorder::Recorder;
use notifico_core::step::SerializedStep;
use notifico_core::templater::RenderedTemplate;
use notifico_core::transport::Transport;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct PushoverCredentials {
    token: String,
}

impl TryFrom<Credential> for PushoverCredentials {
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
                    .strip_prefix("pushover://")
                    .unwrap_or_default()
                    .to_owned(),
            }),
        }
    }
}

impl TypedCredential for PushoverCredentials {
    const TRANSPORT_NAME: &'static str = "pushover";
}

#[derive(Serialize, Deserialize)]
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

pub struct PushoverPlugin {
    client: reqwest::Client,
    credentials: Arc<dyn CredentialStorage>,
    recorder: Arc<dyn Recorder>,
}

impl PushoverPlugin {
    pub fn new(credentials: Arc<dyn CredentialStorage>, recorder: Arc<dyn Recorder>) -> Self {
        Self {
            client: reqwest::Client::new(),
            credentials,
            recorder,
        }
    }
}

#[async_trait]
impl EnginePlugin for PushoverPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::Send { credential } => {
                let credential: PushoverCredentials = self
                    .credentials
                    .resolve(context.project_id, credential)
                    .await?;

                let contact: Vec<PushoverContact> = context.get_recipient()?.get_contacts();

                for contact in contact {
                    for message in context.messages.iter().cloned() {
                        let content: Message = message.content.try_into()?;
                        let request = PushoverMessageRequest {
                            token: credential.token.clone(),
                            user: contact.user.clone(),
                            message: content.text,
                            attachment_base64: None,
                            attachment_type: None,
                            device: None,
                            html: Some(1),
                            priority: None,
                            sound: None,
                            timestamp: None,
                            title: Some(content.title),
                            ttl: None,
                            url: None,
                            url_title: None,
                        };

                        let result = self
                            .client
                            .post("https://api.pushover.net/1/messages.json")
                            .body(serde_urlencoded::to_string(request).unwrap_or_default())
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
                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}

impl Transport for PushoverPlugin {
    fn name(&self) -> Cow<'static, str> {
        "pushover".into()
    }

    fn send_step(&self) -> Cow<'static, str> {
        "pushover.send".into()
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct PushoverContact {
    user: String,
}

impl TryFrom<Contact> for PushoverContact {
    type Error = EngineError;

    fn try_from(value: Contact) -> Result<Self, Self::Error> {
        Ok(Self { user: value.value })
    }
}

impl TypedContact for PushoverContact {
    const CONTACT_TYPE: &'static str = "pushover";
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    pub text: String,
    pub title: String,
}

impl TryFrom<RenderedTemplate> for Message {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        Ok(Self {
            text: value.get("text")?.to_string(),
            title: value.get("title")?.to_string(),
        })
    }
}
