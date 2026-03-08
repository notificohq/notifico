use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::channel::ChannelId;
use crate::error::CoreError;

use super::{
    ContentField, ContentFieldType, ContentSchema, CredentialField, CredentialSchema,
    DeliveryResult, RenderedMessage, Transport,
};

/// Slack transport using the Web API `chat.postMessage` endpoint.
///
/// - recipient_contact: the Slack channel ID or user ID (e.g. "C01234", "U01234")
/// - content: `text` (required), `blocks` (optional JSON array of Block Kit blocks)
/// - credentials: `bot_token` (required, xoxb-...)
pub struct SlackTransport {
    client: Client,
}

impl SlackTransport {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for SlackTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct SlackResponse {
    ok: bool,
    ts: Option<String>,
    error: Option<String>,
}

#[async_trait]
impl Transport for SlackTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("slack")
    }

    fn display_name(&self) -> &str {
        "Slack"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField {
                    name: "text".into(),
                    field_type: ContentFieldType::Text,
                    required: true,
                    description: "Message text (also used as fallback for blocks)".into(),
                },
                ContentField {
                    name: "blocks".into(),
                    field_type: ContentFieldType::Json,
                    required: false,
                    description: "Slack Block Kit blocks (JSON array)".into(),
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
                description: "Slack Bot User OAuth Token (xoxb-...)".into(),
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

        let channel = &message.recipient_contact;

        let mut params = serde_json::json!({
            "channel": channel,
            "text": text,
        });

        if let Some(blocks) = message.content.get("blocks") {
            params["blocks"] = blocks.clone();
        }

        let resp = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(bot_token)
            .json(&params)
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("Slack HTTP error: {e}")))?;

        let status = resp.status();
        let body: SlackResponse = resp
            .json()
            .await
            .map_err(|e| CoreError::Transport(format!("Slack response parse error: {e}")))?;

        if body.ok {
            Ok(DeliveryResult::Delivered {
                provider_message_id: body.ts,
            })
        } else {
            let error = body
                .error
                .unwrap_or_else(|| format!("Slack API error (HTTP {})", status));
            let retryable = matches!(
                error.as_str(),
                "rate_limited" | "service_unavailable" | "internal_error"
            );
            tracing::warn!(
                channel = %channel,
                error = %error,
                retryable = retryable,
                "Slack delivery failed"
            );
            Ok(DeliveryResult::Failed { error, retryable })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slack_schema() {
        let transport = SlackTransport::new();
        assert_eq!(transport.channel_id(), ChannelId::new("slack"));
        assert_eq!(transport.display_name(), "Slack");

        let content = transport.content_schema();
        assert_eq!(content.fields.len(), 2);
        assert_eq!(content.fields[0].name, "text");
        assert!(content.fields[0].required);
        assert_eq!(content.fields[1].name, "blocks");
        assert!(!content.fields[1].required);

        let creds = transport.credential_schema();
        assert_eq!(creds.fields.len(), 1);
        assert_eq!(creds.fields[0].name, "bot_token");
        assert!(creds.fields[0].required);
        assert!(creds.fields[0].secret);
    }

    #[tokio::test]
    async fn slack_missing_credentials() {
        let transport = SlackTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("slack"),
            recipient_contact: "C01234".into(),
            content: serde_json::json!({"text": "Hello!"}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn slack_missing_text() {
        let transport = SlackTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("slack"),
            recipient_contact: "C01234".into(),
            content: serde_json::json!({}),
            credentials: serde_json::json!({"bot_token": "xoxb-test"}),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }
}
