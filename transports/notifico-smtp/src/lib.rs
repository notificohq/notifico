mod context;
mod credentials;
mod headers;
mod templater;

use crate::context::PluginContext;
use crate::headers::ListUnsubscribe;
use crate::templater::RenderedEmail;
use async_trait::async_trait;
use credentials::SmtpServerCredentials;
use lettre::message::header::ContentType;
use lettre::message::{Attachment, SinglePart};
use lettre::{
    message::{Mailbox, MultiPart},
    AsyncSmtpTransport, AsyncTransport, Tokio1Executor,
};
use moka::future::Cache;
use notifico_attachment::AttachmentPlugin;
use notifico_core::credentials::RawCredential;
use notifico_core::error::EngineError;
use notifico_core::pipeline::context::{Message, PipelineContext};
use notifico_core::recipient::{RawContact, TypedContact};
use notifico_core::simpletransport::SimpleTransport;
use serde::Deserialize;
use std::str::FromStr;
use std::sync::Arc;

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
    attachments: Arc<AttachmentPlugin>,
    pools: Cache<String, AsyncSmtpTransport<Tokio1Executor>>,
}

impl EmailTransport {
    pub fn new(attachments: Arc<AttachmentPlugin>) -> Self {
        Self {
            attachments,
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

        let email_message = self.construct_message(contact, message, context).await?;

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

impl EmailTransport {
    async fn construct_message(
        &self,
        contact: RawContact,
        message: Message,
        context: &mut PipelineContext,
    ) -> Result<lettre::Message, EngineError> {
        let contact: EmailContact = contact.try_into()?;
        let rendered: RenderedEmail = message.content.try_into()?;

        let plugin_context: PluginContext =
            serde_json::from_value(context.plugin_contexts.clone().into()).unwrap();

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
            return Ok(builder.body(rendered.body).unwrap());
        }

        let attachments = self
            .attachments
            .get_attachments(&message.attachments)
            .await?;

        let mut attachments_mixed = vec![];
        let mut attachments_inline = vec![];

        for attachment in attachments {
            if attachment.extras.contains_key("email.cid") {
                attachments_inline.push(attachment);
            } else {
                attachments_mixed.push(attachment);
            }
        }

        // multipart/mixed
        let mut mp_mixed = MultiPart::mixed().build();
        if rendered.body_html.is_empty() {
            mp_mixed = mp_mixed.singlepart(SinglePart::plain(rendered.body))
        } else {
            // multipart/alternative
            let mut mp_alternative = MultiPart::alternative().build();
            if attachments_inline.is_empty() {
                mp_alternative = mp_alternative.singlepart(SinglePart::plain(rendered.body));
                mp_alternative = mp_alternative.singlepart(SinglePart::html(rendered.body_html));
            } else {
                mp_alternative = mp_alternative.singlepart(SinglePart::plain(rendered.body));

                // multipart/related
                let mut mp_related = MultiPart::related().build();
                mp_related = mp_related.singlepart(SinglePart::html(rendered.body_html));
                for mut attachment in attachments_inline {
                    let cid = attachment.extras.get("email.cid").unwrap().clone();

                    let content_type = ContentType::parse(attachment.mime_type.as_ref()).unwrap();
                    let attach =
                        Attachment::new_inline(cid).body(attachment.content().await?, content_type);
                    mp_related = mp_related.singlepart(attach);
                }
                mp_alternative = mp_alternative.multipart(mp_related);
            }
            mp_mixed = mp_mixed.multipart(mp_alternative)
        }

        for mut attachment in attachments_mixed {
            let content_type = ContentType::parse(attachment.mime_type.as_ref()).unwrap();
            let attach = Attachment::new(attachment.file_name.clone())
                .body(attachment.content().await?, content_type);
            mp_mixed = mp_mixed.singlepart(attach)
        }

        builder
            .multipart(mp_mixed)
            .map_err(|e| EngineError::InternalError(e.into()))
    }
}
