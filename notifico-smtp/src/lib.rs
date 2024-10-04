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
use notifico_core::credentials::get_typed_credential;
use notifico_core::{
    credentials::Credentials,
    engine::plugin::{EnginePlugin, StepOutput},
    engine::PipelineContext,
    error::EngineError,
    pipeline::SerializedStep,
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
    credentials: Arc<dyn Credentials>,
}

impl EmailPlugin {
    pub fn new(credentials: Arc<dyn Credentials>) -> Self {
        Self { credentials }
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
                let Some(recipient) = context.recipient.clone() else {
                    return Err(EngineError::RecipientNotSet);
                };

                let contact: EmailContact = recipient.get_primary_contact()?;

                let credential: SmtpServerCredentials = get_typed_credential(
                    self.credentials.as_ref(),
                    context.project_id,
                    &credential,
                )
                .await?;

                let transport = {
                    AsyncSmtpTransport::<Tokio1Executor>::from_url(&credential.into_url())
                        .unwrap()
                        .build()
                };

                for message in context.messages.iter().cloned() {
                    let rendered: RenderedEmail = message.try_into().unwrap();

                    let message = {
                        let content: PluginContext =
                            serde_json::from_value(context.plugin_contexts.clone().into()).unwrap();

                        let mut builder = lettre::Message::builder();
                        builder = builder.from(rendered.from);
                        builder = builder.to(contact.address.clone());
                        builder = builder.subject(rendered.subject);
                        if let Some(list_unsubscribe) = content.list_unsubscribe {
                            builder = builder.header(ListUnsubscribe::from(list_unsubscribe));
                        }
                        builder
                            .multipart(MultiPart::alternative_plain_html(
                                rendered.body_plaintext,
                                rendered.body_html,
                            ))
                            .unwrap()
                    };

                    transport.send(message).await.unwrap();
                }
            }
        }

        Ok(StepOutput::Continue)
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}
