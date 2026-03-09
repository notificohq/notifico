use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use notifico_core::channel::ChannelId;
use notifico_core::error::CoreError;
use notifico_core::transport::{
    ContentField, ContentFieldType, ContentSchema, CredentialField, CredentialSchema,
    DeliveryResult, RenderedMessage, Transport,
};

/// Twilio SMS transport.
///
/// - recipient_contact: the destination phone number (E.164 format, e.g. "+15551234567")
/// - content: `text` (required)
/// - credentials: `account_sid` (required), `auth_token` (required), `from_number` (required)
pub struct TwilioSmsTransport {
    client: Client,
}

impl TwilioSmsTransport {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for TwilioSmsTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct TwilioResponse {
    sid: Option<String>,
    #[allow(dead_code)]
    status: Option<String>,
    message: Option<String>,
}

#[async_trait]
impl Transport for TwilioSmsTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("sms")
    }

    fn display_name(&self) -> &str {
        "SMS (Twilio)"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![ContentField {
                name: "text".into(),
                field_type: ContentFieldType::Text,
                required: true,
                description: "SMS message body".into(),
            }],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![
                CredentialField {
                    name: "account_sid".into(),
                    required: true,
                    secret: false,
                    description: "Twilio Account SID".into(),
                },
                CredentialField {
                    name: "auth_token".into(),
                    required: true,
                    secret: true,
                    description: "Twilio Auth Token".into(),
                },
                CredentialField {
                    name: "from_number".into(),
                    required: true,
                    secret: false,
                    description: "Sender phone number (E.164 format)".into(),
                },
            ],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        let account_sid = message
            .credentials
            .get("account_sid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing account_sid credential".into()))?;

        let auth_token = message
            .credentials
            .get("auth_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing auth_token credential".into()))?;

        let from_number = message
            .credentials
            .get("from_number")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing from_number credential".into()))?;

        let text = message
            .content
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing 'text' in content".into()))?;

        let to_number = &message.recipient_contact;

        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{account_sid}/Messages.json"
        );

        let resp = self
            .client
            .post(&url)
            .basic_auth(account_sid, Some(auth_token))
            .form(&[
                ("To", to_number.as_str()),
                ("From", from_number),
                ("Body", text),
            ])
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("Twilio HTTP error: {e}")))?;

        let status = resp.status();
        let body: TwilioResponse = resp
            .json()
            .await
            .map_err(|e| CoreError::Transport(format!("Twilio response parse error: {e}")))?;

        if status.is_success() || status.as_u16() == 201 {
            Ok(DeliveryResult::Delivered {
                provider_message_id: body.sid,
            })
        } else {
            let error = body
                .message
                .unwrap_or_else(|| format!("Twilio API error (HTTP {})", status));
            let retryable = status.is_server_error() || status.as_u16() == 429;
            tracing::warn!(
                to = %to_number,
                status = %status.as_u16(),
                error = %error,
                retryable = retryable,
                "Twilio SMS delivery failed"
            );
            Ok(DeliveryResult::Failed { error, retryable })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twilio_sms_schema() {
        let transport = TwilioSmsTransport::new();
        assert_eq!(transport.channel_id(), ChannelId::new("sms"));
        assert_eq!(transport.display_name(), "SMS (Twilio)");

        let content = transport.content_schema();
        assert_eq!(content.fields.len(), 1);
        assert_eq!(content.fields[0].name, "text");
        assert!(content.fields[0].required);

        let creds = transport.credential_schema();
        assert_eq!(creds.fields.len(), 3);
        assert_eq!(creds.fields[0].name, "account_sid");
        assert!(!creds.fields[0].secret);
        assert_eq!(creds.fields[1].name, "auth_token");
        assert!(creds.fields[1].secret);
        assert_eq!(creds.fields[2].name, "from_number");
        assert!(!creds.fields[2].secret);
    }

    #[tokio::test]
    async fn twilio_missing_credentials() {
        let transport = TwilioSmsTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("sms"),
            recipient_contact: "+15551234567".into(),
            content: serde_json::json!({"text": "Hello!"}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn twilio_missing_text() {
        let transport = TwilioSmsTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("sms"),
            recipient_contact: "+15551234567".into(),
            content: serde_json::json!({}),
            credentials: serde_json::json!({
                "account_sid": "AC123",
                "auth_token": "token",
                "from_number": "+15559876543"
            }),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }
}
