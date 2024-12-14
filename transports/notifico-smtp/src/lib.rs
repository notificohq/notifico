mod context;
mod credentials;
mod headers;
mod templater;

use crate::context::PluginContext;
use crate::headers::ListUnsubscribe;
use crate::templater::RenderedEmail;
use async_trait::async_trait;
use credentials::SmtpServerCredentials;
use lettre::{
    message::{Mailbox, MultiPart},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use moka::future::Cache;
use notifico_core::contact::{RawContact, TypedContact};
use notifico_core::credentials::RawCredential;
use notifico_core::engine::Message;
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::{engine::PipelineContext, error::EngineError};
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct EmailContact {
    address: Mailbox,
}

impl TryFrom<RawContact> for EmailContact {
    type Error = EngineError;

    fn try_from(value: RawContact) -> Result<Self, Self::Error> {
        Ok(Self {
            address: Mailbox::from_str(&value.value)
                .map_err(|e| EngineError::InvalidContactFormat(e.to_string()))?,
        })
    }
}

impl TypedContact for EmailContact {
    const CONTACT_TYPE: &'static str = "email";
}

pub struct EmailTransport {
    pools: Cache<String, AsyncSmtpTransport<Tokio1Executor>>,
}

impl EmailTransport {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
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
impl SimpleTransport for EmailTransport {
    async fn send_message(
        &self,
        credential: RawCredential,
        contact: RawContact,
        message: Message,
        context: &mut PipelineContext,
    ) -> Result<(), EngineError> {
        let credential: SmtpServerCredentials = credential.try_into()?;
        let contact: EmailContact = contact.try_into()?;
        let rendered: RenderedEmail = message.content.try_into()?;

        let plugin_context: PluginContext =
            serde_json::from_value(context.plugin_contexts.clone().into()).unwrap();

        let email_message = {
            let mut builder = lettre::Message::builder();
            builder = builder.from(rendered.from);
            builder = builder.to(contact.address.clone());
            builder = builder.subject(rendered.subject);
            if let Some(list_unsubscribe) = plugin_context.list_unsubscribe.clone() {
                builder = builder.header(ListUnsubscribe::from(list_unsubscribe));
            }

            if rendered.body_html.is_empty()
                && !rendered.body.is_empty()
                && message.attachments.is_empty()
            {
                builder.body(rendered.body).unwrap()
            } else {
                let multipart =
                    MultiPart::alternative_plain_html(rendered.body, rendered.body_html);
                builder.multipart(multipart).unwrap()
            }
        };

        let transport = self.get_transport(credential).await;
        let _result = transport
            .send(email_message)
            .await
            .map_err(|e| EngineError::InternalError(e.into()))?;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "smtp"
    }

    fn supports_contact(&self, r#type: &str) -> bool {
        r#type == "email"
    }
}
