mod credentials;
mod step;

use async_trait::async_trait;
use credentials::SmtpServerCredentials;
use lettre::{
    message::{Mailbox, MultiPart},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use notifico_core::engine::plugin::EnginePlugin;
use notifico_core::{
    credentials::Credentials,
    engine::PipelineContext,
    error::EngineError,
    pipeline::SerializedStep,
    recipient::Contact,
    templater::{RenderResponse, Templater},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;
use step::{CredentialSelector, EmailStep};
use uuid::Uuid;

const CHANNEL_NAME: &'static str = "email";

#[derive(Debug, Deserialize)]
pub struct EmailContact {
    address: Mailbox,
}

impl TryFrom<Contact> for EmailContact {
    type Error = EngineError;

    fn try_from(value: Contact) -> Result<Self, Self::Error> {
        serde_json::from_value(value.into_json()).map_err(|_| EngineError::InvalidContactFormat)
    }
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

#[derive(Default, Serialize, Deserialize)]
struct TelegramContext {
    template_id: Option<Uuid>,
}

#[async_trait]
impl EnginePlugin for EmailPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<(), EngineError> {
        let telegram_step: EmailStep = step.clone().try_into().unwrap();

        match telegram_step {
            EmailStep::LoadTemplate { template_id } => {
                let rendered_template = self
                    .templater
                    .render("email", template_id, context.event_context.0.clone())
                    .await?;

                let rendered_template: RenderedEmail = rendered_template.try_into().unwrap();

                context.plugin_contexts.insert(
                    "email.content".into(),
                    serde_json::to_value(rendered_template).unwrap(),
                );
            }
            EmailStep::Send(cred_selector) => {
                let Some(recipient) = context.recipient.clone() else {
                    return Err(EngineError::RecipientNotSet);
                };

                let contact =
                    EmailContact::try_from(recipient.get_primary_contact("email")?.clone())?;

                let message = {
                    let content = context
                        .plugin_contexts
                        .get("email.content")
                        .ok_or(EngineError::TemplateNotSet)?;
                    let content: RenderedEmail = serde_json::from_value(content.clone()).unwrap();

                    let mut builder = lettre::Message::builder();
                    builder = builder.from(Mailbox::from_str(&content.from).unwrap());
                    builder = builder.to(contact.address);
                    builder = builder.subject(content.subject);
                    builder
                        .multipart(MultiPart::alternative_plain_html(
                            content.body_plaintext,
                            content.body_html,
                        ))
                        .unwrap()
                };

                let smtpcred: SmtpServerCredentials = match cred_selector {
                    CredentialSelector::SmtpName { smtp_name } => self
                        .credentials
                        .get_credential(context.project_id, "smtp_server", &smtp_name)?
                        .try_into()?,
                };

                let transport = {
                    AsyncSmtpTransport::<Tokio1Executor>::from_url(&smtpcred.into_url())
                        .unwrap()
                        .build()
                };

                transport.send(message).await.unwrap();
            }
        }

        Ok(())
    }

    fn step_namespace(&self) -> Cow<'static, str> {
        "email".into()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RenderedEmail {
    headers: String,

    from: String,

    subject: String,

    body_html: String,
    body_plaintext: String,
}

impl TryFrom<RenderResponse> for RenderedEmail {
    type Error = ();

    fn try_from(value: RenderResponse) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0)).map_err(|_| ())
    }
}
