mod credentials;
mod slackapi;

use async_trait::async_trait;
use credentials::SlackCredentials;
use notifico_attachment::AttachmentPlugin;
use notifico_core::credentials::RawCredential;
use notifico_core::error::EngineError;
use notifico_core::pipeline::context::{Message, PipelineContext};
use notifico_core::recipient::{RawContact, TypedContact};
use notifico_core::simpletransport::SimpleTransport;
use notifico_core::templater::RenderedTemplate;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;

pub struct SlackTransport {
    client: slackapi::SlackApi,
    attachments: Arc<AttachmentPlugin>,
}

impl SlackTransport {
    pub fn new(client: reqwest::Client, attachments: Arc<AttachmentPlugin>) -> Self {
        SlackTransport {
            client: slackapi::SlackApi::new(client),
            attachments,
        }
    }
}

#[async_trait]
impl SimpleTransport for SlackTransport {
    async fn send_message(
        &self,
        credential: RawCredential,
        contact: RawContact,
        message: Message,
        _context: &mut PipelineContext,
    ) -> Result<(), EngineError> {
        let credential: SlackCredentials = credential.try_into()?;
        let contact: SlackContact = contact.try_into()?;
        let content: SlackMessage = message.content.try_into()?;

        let slack_message = slackapi::SlackMessage::Text {
            channel: contact.channel_id.clone(),
            text: content.body,
        };

        self.client
            .chat_post_message(&credential.token, slack_message)
            .await
            .map_err(|e| EngineError::InternalError(e.into()))?;

        for attachment in message.attachments {
            let file = self.attachments.get_attachment(&attachment).await?;

            self.client
                .upload_file(
                    &credential.token,
                    file.file,
                    &file.file_name,
                    file.size,
                    &contact.channel_id,
                )
                .await
                .map_err(|e| EngineError::InternalError(e.into()))?;
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "slack"
    }

    fn supports_contact(&self, r#type: &str) -> bool {
        r#type == "slack"
    }

    fn supported_channels(&self) -> Vec<Cow<'static, str>> {
        vec!["slack".into()]
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct SlackContact {
    channel_id: String,
}

impl TryFrom<RawContact> for SlackContact {
    type Error = EngineError;

    fn try_from(value: RawContact) -> Result<Self, Self::Error> {
        Ok(Self {
            channel_id: value.value,
        })
    }
}

impl TypedContact for SlackContact {
    const CONTACT_TYPE: &'static str = "slack";
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SlackMessage {
    pub body: String,
}

impl TryFrom<RenderedTemplate> for SlackMessage {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        Ok(Self {
            body: value.get("body")?.to_string(),
        })
    }
}
