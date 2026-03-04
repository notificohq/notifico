pub mod email;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::channel::ChannelId;
use crate::error::CoreError;

/// Schema describing what fields a channel needs in template content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSchema {
    pub fields: Vec<ContentField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentField {
    pub name: String,
    pub field_type: ContentFieldType,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentFieldType {
    Text,
    Html,
    Json,
}

/// Schema describing what credentials a transport needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSchema {
    pub fields: Vec<CredentialField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialField {
    pub name: String,
    pub required: bool,
    pub secret: bool,
    pub description: String,
}

/// A fully rendered message ready for delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedMessage {
    pub channel: ChannelId,
    pub recipient_contact: String,
    pub content: Value,
    pub credentials: Value,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub disposition: AttachmentDisposition,
    pub content_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentDisposition {
    Inline,
    Attachment,
}

/// Result of a delivery attempt.
#[derive(Debug, Clone)]
pub enum DeliveryResult {
    Delivered {
        provider_message_id: Option<String>,
    },
    Failed {
        error: String,
        retryable: bool,
    },
}

/// The core transport trait. All channels implement this.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Unique channel identifier.
    fn channel_id(&self) -> ChannelId;

    /// Human-readable name for admin UI.
    fn display_name(&self) -> &str;

    /// Schema for template content fields.
    fn content_schema(&self) -> ContentSchema;

    /// Schema for required credentials.
    fn credential_schema(&self) -> CredentialSchema;

    /// Send a rendered message.
    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError>;
}

/// A transport that logs messages to stdout. Useful for development and testing.
pub struct ConsoleTransport;

#[async_trait]
impl Transport for ConsoleTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("console")
    }

    fn display_name(&self) -> &str {
        "Console (stdout)"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![ContentField {
                name: "text".into(),
                field_type: ContentFieldType::Text,
                required: true,
                description: "Message text to print".into(),
            }],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema { fields: vec![] }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        let text = message
            .content
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("[no text field]");

        tracing::info!(
            channel = %message.channel,
            recipient = %message.recipient_contact,
            text = %text,
            "Console transport: delivering message"
        );

        Ok(DeliveryResult::Delivered {
            provider_message_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn console_transport_sends_ok() {
        let transport = ConsoleTransport;
        let message = RenderedMessage {
            channel: ChannelId::new("console"),
            recipient_contact: "user@example.com".into(),
            content: serde_json::json!({"text": "Hello, world!"}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };

        let result = transport.send(&message).await.unwrap();
        match result {
            DeliveryResult::Delivered { .. } => {}
            DeliveryResult::Failed { .. } => panic!("Expected Delivered"),
        }
    }

    #[test]
    fn console_transport_schema() {
        let transport = ConsoleTransport;
        assert_eq!(transport.channel_id(), ChannelId::new("console"));
        assert_eq!(transport.display_name(), "Console (stdout)");
        assert_eq!(transport.content_schema().fields.len(), 1);
        assert_eq!(transport.content_schema().fields[0].name, "text");
        assert!(transport.credential_schema().fields.is_empty());
    }
}
