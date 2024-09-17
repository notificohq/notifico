mod step;

use async_trait::async_trait;
use lettre::message::header::{Header, HeaderValue, Headers, Subject};
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::Credentials as LettreCredentials;
use lettre::transport::smtp::client::TlsParameters;
use lettre::{
    Address, AsyncSmtpTransport, AsyncTransport, SmtpTransport, Tokio1Executor, Transport,
};
use notifico_core::credentials::Credentials;
use notifico_core::engine::{EngineError, EnginePlugin, PipelineContext};
use notifico_core::pipeline::SerializedStep;
use notifico_core::recipient::Contact;
use notifico_core::templater::{RenderResponse, Templater};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;
use step::{CredentialSelector, EmailStep};
use tracing::debug;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct EmailContact {
    address: Mailbox,
}

impl TryFrom<Contact> for EmailContact {
    type Error = ();

    fn try_from(value: Contact) -> Result<Self, Self::Error> {
        serde_json::from_value(value.into_json()).map_err(|_| ())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SmtpServerCredentials {
    tls: bool,
    host: String,
    port: Option<u16>,
    username: String,
    password: String,
}

impl SmtpServerCredentials {
    pub fn into_url(self) -> String {
        let (protocol, port, tls_param) = match self.tls {
            true => ("smtps", 465, "?tls=required"),
            false => ("smtp", 25, ""),
        };

        let port = self.port.unwrap_or(port);

        format!(
            "{protocol}://{}:{}@{}:{port}{tls_param}",
            self.username, self.password, self.host
        )
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
        let plugin_context = context
            .plugin_contexts
            .entry("email".into())
            .or_insert(Value::Object(Default::default()));

        debug!("Plugin context: {:?}", plugin_context);

        let mut plugin_context: TelegramContext =
            serde_json::from_value(plugin_context.clone()).unwrap();
        let telegram_step: EmailStep = step.clone().try_into().unwrap();

        match telegram_step {
            EmailStep::LoadTemplate { template_id } => {
                plugin_context.template_id = Some(template_id);
                context.plugin_contexts.insert(
                    "email".into(),
                    serde_json::to_value(plugin_context).unwrap(),
                );
            }
            EmailStep::Send(cred_selector) => {
                let Some(template_id) = plugin_context.template_id else {
                    return Err(EngineError::PipelineInterrupted);
                };

                let credentials = match cred_selector {
                    CredentialSelector::SmtpName { smtp_name } => self
                        .credentials
                        .get_credential("smtp_server", &smtp_name)
                        .unwrap(),
                };

                let smtpcred: SmtpServerCredentials = serde_json::from_value(credentials).unwrap();

                let transport = {
                    AsyncSmtpTransport::<Tokio1Executor>::from_url(&smtpcred.into_url())
                        .unwrap()
                        .build()
                };

                let rendered_template = self
                    .templater
                    .render("email", template_id, context.event_context.0.clone())
                    .await
                    .unwrap();

                let rendered_template: RenderedEmail = rendered_template.try_into().unwrap();

                let contact = EmailContact::try_from(
                    context
                        .recipient
                        .clone()
                        .unwrap()
                        .get_primary_contact("email")
                        .ok_or(EngineError::PipelineInterrupted)
                        .cloned()?,
                )
                .unwrap();

                let mut builder = lettre::Message::builder();
                builder = builder.from(Mailbox::from_str(&rendered_template.from).unwrap());
                builder = builder.to(contact.address);
                builder = builder.subject(rendered_template.subject);

                let message = builder
                    .multipart(MultiPart::alternative_plain_html(
                        rendered_template.body_plaintext,
                        rendered_template.body_html,
                    ))
                    .unwrap();

                transport.send(message).await.unwrap();
            }
        }

        Ok(())
    }

    fn step_type(&self) -> Cow<'static, str> {
        "email".into()
    }
}

#[derive(Deserialize, Clone)]
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
