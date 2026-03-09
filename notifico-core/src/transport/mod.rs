pub mod discord;
pub mod email;
pub mod slack;
pub mod sms_twilio;
pub mod telegram;
pub mod webhook;

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

