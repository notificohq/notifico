use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::channel::ChannelId;
use crate::error::CoreError;

use super::{
    ContentField, ContentFieldType, ContentSchema, CredentialField, CredentialSchema,
    DeliveryResult, RenderedMessage, Transport,
};

/// Telegram Bot API transport.
///
/// Sends messages via the `sendMessage` endpoint.
/// - recipient_contact: the Telegram chat ID
/// - content: `text` (required), `parse_mode` (optional: "HTML" or "MarkdownV2")
/// - credentials: `bot_token` (required)
pub struct TelegramTransport {
    client: Client,
}

impl TelegramTransport {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for TelegramTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct TelegramResponse {
    ok: bool,
    result: Option<TelegramMessage>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct TelegramMessage {
    message_id: i64,
}

#[async_trait]
impl Transport for TelegramTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("telegram")
    }

    fn display_name(&self) -> &str {
        "Telegram"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField {
                    name: "text".into(),
                    field_type: ContentFieldType::Text,
                    required: true,
                    description: "Message text".into(),
                },
                ContentField {
                    name: "parse_mode".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "Parse mode: HTML or MarkdownV2 (optional)".into(),
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
                description: "Telegram Bot API token from @BotFather".into(),
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

        let chat_id = &message.recipient_contact;

        let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");

        let mut params = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
        });

        if let Some(parse_mode) = message.content.get("parse_mode").and_then(|v| v.as_str()) {
            params["parse_mode"] = serde_json::Value::String(parse_mode.to_string());
        }

        let resp = self
            .client
            .post(&url)
            .json(&params)
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("Telegram HTTP error: {e}")))?;

        let status = resp.status();
        let body: TelegramResponse = resp
            .json()
            .await
            .map_err(|e| CoreError::Transport(format!("Telegram response parse error: {e}")))?;

        if body.ok {
            let message_id = body.result.map(|m| m.message_id.to_string());
            Ok(DeliveryResult::Delivered {
                provider_message_id: message_id,
            })
        } else {
            let error = body
                .description
                .unwrap_or_else(|| format!("Telegram API error (HTTP {})", status));
            let retryable = status.is_server_error() || status.as_u16() == 429;
            tracing::warn!(
                chat_id = %chat_id,
                error = %error,
                retryable = retryable,
                "Telegram delivery failed"
            );
            Ok(DeliveryResult::Failed { error, retryable })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telegram_schema() {
        let transport = TelegramTransport::new();
        assert_eq!(transport.channel_id(), ChannelId::new("telegram"));
        assert_eq!(transport.display_name(), "Telegram");

        let content = transport.content_schema();
        assert_eq!(content.fields.len(), 2);
        assert_eq!(content.fields[0].name, "text");
        assert!(content.fields[0].required);
        assert_eq!(content.fields[1].name, "parse_mode");
        assert!(!content.fields[1].required);

        let creds = transport.credential_schema();
        assert_eq!(creds.fields.len(), 1);
        assert_eq!(creds.fields[0].name, "bot_token");
        assert!(creds.fields[0].required);
        assert!(creds.fields[0].secret);
    }

    #[tokio::test]
    async fn telegram_missing_credentials() {
        let transport = TelegramTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("telegram"),
            recipient_contact: "12345".into(),
            content: serde_json::json!({"text": "Hello!"}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }
}
