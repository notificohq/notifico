mod context;
mod credentials;
mod headers;
mod step;
mod templater;

use crate::context::{Email, EMAIL_BODY_HTML, EMAIL_BODY_PLAINTEXT, EMAIL_FROM, EMAIL_SUBJECT};
use crate::headers::ListUnsubscribe;
use crate::step::STEPS;
use crate::templater::RenderedEmail;
use async_trait::async_trait;
use credentials::SmtpServerCredentials;
use lettre::{
    message::{Mailbox, MultiPart},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use notifico_core::{
    credentials::Credentials,
    credentials::TypedCredential,
    engine::plugin::{EnginePlugin, StepOutput},
    engine::PipelineContext,
    error::EngineError,
    pipeline::SerializedStep,
    recipient::TypedContact,
    templater::Templater,
};
use serde::Deserialize;
use std::borrow::Cow;
use std::sync::Arc;
use step::Step;

const CHANNEL_NAME: &'static str = "email";

#[derive(Debug, Deserialize)]
pub struct EmailContact {
    address: Mailbox,
}

impl TypedContact for EmailContact {
    const CONTACT_TYPE: &'static str = "email";
}

pub struct EmailPlugin {
    templater: Arc<dyn Templater>,
    credentials: Arc<dyn Credentials>,
}

impl EmailPlugin {
    pub fn new(templater: Arc<dyn Templater>, credentials: Arc<dyn Credentials>) -> Self {
        Self {
            templater,
            credentials,
        }
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
            Step::LoadTemplate { template_id } => {
                let rendered_template = self
                    .templater
                    .render(CHANNEL_NAME, template_id, context.event_context.0.clone())
                    .await?;

                let rendered_template: RenderedEmail = rendered_template.try_into().unwrap();

                context.plugin_contexts.insert(
                    EMAIL_FROM.into(),
                    serde_json::to_value(rendered_template.from).unwrap(),
                );
                context.plugin_contexts.insert(
                    EMAIL_SUBJECT.into(),
                    serde_json::to_value(rendered_template.subject).unwrap(),
                );
                context.plugin_contexts.insert(
                    EMAIL_BODY_HTML.into(),
                    serde_json::to_value(rendered_template.body_html).unwrap(),
                );
                context.plugin_contexts.insert(
                    EMAIL_BODY_PLAINTEXT.into(),
                    serde_json::to_value(rendered_template.body_plaintext).unwrap(),
                );
            }
            Step::Send { credential } => {
                let Some(recipient) = context.recipient.clone() else {
                    return Err(EngineError::RecipientNotSet);
                };

                let contact: EmailContact = recipient.get_primary_contact()?;

                let message = {
                    let content: Email =
                        serde_json::from_value(context.plugin_contexts.clone().into()).unwrap();

                    let mut builder = lettre::Message::builder();
                    builder = builder.from(content.from);
                    builder = builder.to(contact.address);
                    builder = builder.subject(content.subject);
                    if let Some(list_unsubscribe) = content.list_unsubscribe {
                        builder = builder.header(ListUnsubscribe::from(list_unsubscribe));
                    }
                    builder
                        .multipart(MultiPart::alternative_plain_html(
                            content.body_plaintext,
                            content.body_html,
                        ))
                        .unwrap()
                };

                let smtpcred: SmtpServerCredentials = self
                    .credentials
                    .get_credential(
                        context.project_id,
                        SmtpServerCredentials::CREDENTIAL_TYPE,
                        &credential,
                    )?
                    .into_typed()?;

                let transport = {
                    AsyncSmtpTransport::<Tokio1Executor>::from_url(&smtpcred.into_url())
                        .unwrap()
                        .build()
                };

                transport.send(message).await.unwrap();
            }
        }

        Ok(StepOutput::None)
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}
