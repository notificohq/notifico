mod context;
mod credentials;
mod headers;
mod step;
mod templater;

use crate::context::PluginContext;
use crate::headers::ListUnsubscribe;
use crate::step::STEPS;
use crate::templater::RenderedEmail;
use async_trait::async_trait;
use credentials::SmtpServerCredentials;
use lettre::{
    message::{Mailbox, MultiPart},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use moka::future::Cache;
use notifico_core::recorder::Recorder;
use notifico_core::step::SerializedStep;
use notifico_core::{
    credentials::CredentialStorage,
    engine::{EnginePlugin, PipelineContext, StepOutput},
    error::EngineError,
    recipient::TypedContact,
};
use serde::Deserialize;
use std::borrow::Cow;
use std::sync::Arc;
use step::Step;

#[derive(Debug, Deserialize)]
pub struct EmailContact {
    address: Mailbox,
}

impl TypedContact for EmailContact {
    const CONTACT_TYPE: &'static str = "email";
}

pub struct EmailPlugin {
    credentials: Arc<dyn CredentialStorage>,
    recorder: Arc<dyn Recorder>,
    pools: Cache<String, AsyncSmtpTransport<Tokio1Executor>>,
}

impl EmailPlugin {
    pub fn new(credentials: Arc<dyn CredentialStorage>, recorder: Arc<dyn Recorder>) -> Self {
        Self {
            credentials,
            recorder,
            pools: Cache::new(100),
        }
    }

    pub async fn get_transport(
        &self,
        credential: SmtpServerCredentials,
    ) -> AsyncSmtpTransport<Tokio1Executor> {
        let cred_url = credential.into_url();
        let transport = self.pools.get(&cred_url).await.unwrap_or_else(|| {
            AsyncSmtpTransport::<Tokio1Executor>::from_url(&cred_url)
                .unwrap()
                .build()
        });

        self.pools.insert(cred_url, transport.clone()).await;
        transport
    }
}

#[async_trait]
impl EnginePlugin for EmailPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::Send { credential } => {
                let contact: EmailContact = context.get_contact()?;

                let credential: SmtpServerCredentials = self
                    .credentials
                    .resolve(context.project_id, credential)
                    .await?;

                let transport = self.get_transport(credential).await;

                let plugin_context: PluginContext =
                    serde_json::from_value(context.plugin_contexts.clone().into()).unwrap();

                for message in context.messages.iter().cloned() {
                    let rendered: RenderedEmail = message.content.try_into()?;

                    let email_message = {
                        let mut builder = lettre::Message::builder();
                        builder = builder.from(rendered.from);
                        builder = builder.to(contact.address.clone());
                        builder = builder.subject(rendered.subject);
                        if let Some(list_unsubscribe) = plugin_context.list_unsubscribe.clone() {
                            builder = builder.header(ListUnsubscribe::from(list_unsubscribe));
                        }
                        builder
                            .multipart(MultiPart::alternative_plain_html(
                                rendered.body_plaintext,
                                rendered.body_html,
                            ))
                            .unwrap()
                    };

                    let result = transport.send(email_message).await;
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

        Ok(StepOutput::Continue)
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}
