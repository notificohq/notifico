use crate::credentials::GotifyCredentials;
use async_trait::async_trait;
use notifico_core::credentials::{CredentialSelector, CredentialStorage, TypedCredential};
use notifico_core::engine::{EnginePlugin, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::recorder::Recorder;
use notifico_core::step::SerializedStep;
use notifico_core::templater::RenderedTemplate;
use notifico_core::transport::Transport;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::borrow::Cow;
use std::sync::Arc;

mod credentials;

#[derive(Serialize)]
struct GotifyRequest {
    title: Option<String>,
    message: String,
    priority: Option<i8>,
    extras: Option<Map<String, Value>>,
}

pub struct GotifyPlugin {
    credentials: Arc<dyn CredentialStorage>,
    recorder: Arc<dyn Recorder>,
    client: reqwest::Client,
}

impl GotifyPlugin {
    pub fn new(credentials: Arc<dyn CredentialStorage>, recorder: Arc<dyn Recorder>) -> Self {
        Self {
            credentials,
            recorder,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl EnginePlugin for GotifyPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::Send { credential } => {
                let credential: GotifyCredentials = self
                    .credentials
                    .resolve(context.project_id, credential)
                    .await?;

                for message in context.messages.iter().cloned() {
                    let content: GotifyContent = message.content.try_into().unwrap();

                    let request = GotifyRequest {
                        title: content.title,
                        message: content.body,
                        priority: None,
                        extras: None,
                    };

                    let result = self
                        .client
                        .post(credential.url.clone())
                        .json(&request)
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
                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        ["gotify.send"].into_iter().map(|s| s.into()).collect()
    }
}

impl Transport for GotifyPlugin {
    fn name(&self) -> Cow<'static, str> {
        "gotify".into()
    }

    fn send_step(&self) -> Cow<'static, str> {
        "gotify.send".into()
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "gotify.send")]
    Send { credential: CredentialSelector },
}

pub(crate) const STEPS: &[&str] = &["gotify.send"];

#[derive(Serialize, Deserialize, Clone)]
struct GotifyContent {
    title: Option<String>,
    body: String,
}

impl TryFrom<RenderedTemplate> for GotifyContent {
    type Error = ();

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0)).map_err(|_| ())
    }
}
