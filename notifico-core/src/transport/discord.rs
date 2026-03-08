use async_trait::async_trait;
use reqwest::Client;

use crate::channel::ChannelId;
use crate::error::CoreError;

use super::{
    ContentField, ContentFieldType, ContentSchema, CredentialField, CredentialSchema,
    DeliveryResult, RenderedMessage, Transport,
};

/// Discord transport using the Bot API to send messages to channels.
///
/// - recipient_contact: the Discord channel ID (snowflake)
/// - content: `text` (required), `embeds` (optional JSON array of embed objects)
/// - credentials: `bot_token` (required)
pub struct DiscordTransport {
    client: Client,
}

impl DiscordTransport {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for DiscordTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for DiscordTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("discord")
    }

    fn display_name(&self) -> &str {
        "Discord"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField {
                    name: "text".into(),
                    field_type: ContentFieldType::Text,
                    required: true,
                    description: "Message content text".into(),
                },
                ContentField {
                    name: "embeds".into(),
                    field_type: ContentFieldType::Json,
                    required: false,
                    description: "Discord embed objects (JSON array)".into(),
                },
            ],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![CredentialField {
                name: "bot_token".into(),
                required: true,
                secret: true,
                description: "Discord Bot token".into(),
            }],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        let bot_token = message
            .credentials
            .get("bot_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing bot_token credential".into()))?;

        let text = message
            .content
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing 'text' in content".into()))?;

        let discord_channel_id = &message.recipient_contact;

        let url = format!(
            "https://discord.com/api/v10/channels/{discord_channel_id}/messages"
        );

        let mut params = serde_json::json!({
            "content": text,
        });

        if let Some(embeds) = message.content.get("embeds") {
            params["embeds"] = embeds.clone();
        }

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {bot_token}"))
            .json(&params)
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("Discord HTTP error: {e}")))?;

        let status = resp.status();

        if status.is_success() {
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| CoreError::Transport(format!("Discord response parse error: {e}")))?;

            let message_id = body.get("id").and_then(|v| v.as_str()).map(String::from);

            Ok(DeliveryResult::Delivered {
                provider_message_id: message_id,
            })
        } else {
            let error_text = resp.text().await.unwrap_or_default();
            let retryable = status.is_server_error() || status.as_u16() == 429;
            tracing::warn!(
                channel_id = %discord_channel_id,
                status = %status.as_u16(),
                error = %error_text,
                retryable = retryable,
                "Discord delivery failed"
            );
            Ok(DeliveryResult::Failed {
                error: format!("Discord API error (HTTP {}): {error_text}", status.as_u16()),
                retryable,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discord_schema() {
        let transport = DiscordTransport::new();
        assert_eq!(transport.channel_id(), ChannelId::new("discord"));
        assert_eq!(transport.display_name(), "Discord");

        let content = transport.content_schema();
        assert_eq!(content.fields.len(), 2);
        assert_eq!(content.fields[0].name, "text");
        assert!(content.fields[0].required);
        assert_eq!(content.fields[1].name, "embeds");
        assert!(!content.fields[1].required);

        let creds = transport.credential_schema();
        assert_eq!(creds.fields.len(), 1);
        assert_eq!(creds.fields[0].name, "bot_token");
        assert!(creds.fields[0].required);
        assert!(creds.fields[0].secret);
    }

    #[tokio::test]
    async fn discord_missing_credentials() {
        let transport = DiscordTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("discord"),
            recipient_contact: "123456789".into(),
            content: serde_json::json!({"text": "Hello!"}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn discord_missing_text() {
        let transport = DiscordTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("discord"),
            recipient_contact: "123456789".into(),
            content: serde_json::json!({}),
            credentials: serde_json::json!({"bot_token": "test-token"}),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }
}
