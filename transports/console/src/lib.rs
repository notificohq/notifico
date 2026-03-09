use async_trait::async_trait;
use notifico_core::channel::ChannelId;
use notifico_core::error::CoreError;
use notifico_core::transport::{
    ContentField, ContentFieldType, ContentSchema, CredentialSchema, DeliveryResult,
    RenderedMessage, Transport,
};

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
