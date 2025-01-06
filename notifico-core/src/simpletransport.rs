use crate::credentials::{CredentialSelector, CredentialStorage, RawCredential};
use crate::engine::{EnginePlugin, StepOutput};
use crate::error::EngineError;
use crate::pipeline::context::{Message, PipelineContext};
use crate::recipient::RawContact;
use crate::recorder::Recorder;
use crate::step::SerializedStep;
use crate::transport::Transport;
use async_trait::async_trait;
use std::borrow::Cow;
use std::sync::Arc;

#[async_trait]
pub trait SimpleTransport: Send + Sync {
    async fn send_message(
        &self,
        credential: RawCredential,
        contact: RawContact,
        message: Message,
        context: &mut PipelineContext,
    ) -> Result<(), EngineError>;

    fn name(&self) -> &'static str;

    fn has_contacts(&self) -> bool {
        true
    }

    fn supports_contact(&self, r#type: &str) -> bool;
}

pub struct SimpleTransportWrapper {
    credentials: Arc<dyn CredentialStorage>,
    recorder: Arc<dyn Recorder>,
    inner: Arc<dyn SimpleTransport>,
}

impl SimpleTransportWrapper {
    pub fn new(
        inner: Arc<dyn SimpleTransport>,
        credentials: Arc<dyn CredentialStorage>,
        recorder: Arc<dyn Recorder>,
    ) -> Self {
        Self {
            inner,
            credentials,
            recorder,
        }
    }
}

#[async_trait]
impl EnginePlugin for SimpleTransportWrapper {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step_type = step.get_type();
        assert_eq!(
            step_type,
            self.send_step(),
            "will fail on searching for suitable plugins"
        );

        let credential_selector = step
            .0
            .get("credential")
            .ok_or(EngineError::MissingCredential)?;
        let credential_selector: CredentialSelector =
            serde_json::from_value(credential_selector.clone())
                .map_err(EngineError::InvalidStep)?;

        let credential = self
            .credentials
            .get_credential(context.project_id, &credential_selector)
            .await?;

        if credential.transport != self.name() {
            return Err(EngineError::InvalidCredentialFormat);
        }

        let contacts = if self.inner.has_contacts() {
            context.get_recipient()?.contacts.clone()
        } else {
            vec![RawContact {
                r#type: String::default(),
                value: String::default(),
            }]
        };

        for contact in contacts {
            if self.inner.has_contacts() && !self.inner.supports_contact(&contact.r#type) {
                continue;
            }
            for message in &context.messages.clone() {
                let result = self
                    .inner
                    .send_message(
                        credential.clone(),
                        contact.clone(),
                        message.clone(),
                        context,
                    )
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

    fn steps(&self) -> Vec<Cow<'static, str>> {
        vec![self.send_step()]
    }
}

impl Transport for SimpleTransportWrapper {
    fn name(&self) -> Cow<'static, str> {
        self.inner.name().into()
    }

    fn send_step(&self) -> Cow<'static, str> {
        let transport = self.inner.name().to_owned();
        Cow::Owned(transport + ".send")
    }
}
