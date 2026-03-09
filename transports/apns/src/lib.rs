use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;

use notifico_core::channel::ChannelId;
use notifico_core::error::CoreError;
use notifico_core::transport::{
    ContentField, ContentFieldType, ContentSchema, CredentialField, CredentialSchema,
    DeliveryResult, RenderedMessage, Transport,
};

/// APNs push notification transport.
///
/// Sends push notifications via the Apple Push Notification service HTTP/2 API
/// using token-based (p8) authentication.
/// - recipient_contact: APNs device token (hex string)
/// - content: `title` (required), `body` (required), `badge` (optional),
///   `sound` (optional), `data` (optional JSON), `category` (optional)
/// - credentials: `team_id`, `key_id`, `private_key` (p8), `environment`
pub struct ApnsTransport {
    client: Client,
}

impl ApnsTransport {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for ApnsTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// APNs error response body.
#[derive(Deserialize)]
struct ApnsErrorResponse {
    reason: Option<String>,
}

/// Non-retryable APNs error reasons.
const NON_RETRYABLE_REASONS: &[&str] = &[
    "BadDeviceToken",
    "Unregistered",
    "DeviceTokenNotForTopic",
];

impl ApnsTransport {
    /// Generate a signed JWT for APNs token-based authentication.
    fn generate_token(
        &self,
        team_id: &str,
        key_id: &str,
        private_key: &str,
    ) -> Result<String, CoreError> {
        let now = Utc::now().timestamp();

        let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::ES256);
        header.kid = Some(key_id.to_string());

        let claims = serde_json::json!({
            "iss": team_id,
            "iat": now,
        });

        let key = jsonwebtoken::EncodingKey::from_ec_pem(private_key.as_bytes())
            .map_err(|e| CoreError::InvalidConfig(format!("Invalid APNs private key: {e}")))?;

        jsonwebtoken::encode(&header, &claims, &key)
            .map_err(|e| CoreError::InvalidConfig(format!("APNs JWT encoding error: {e}")))
    }

    /// Return the APNs endpoint URL based on environment.
    fn endpoint(environment: &str) -> &'static str {
        if environment == "production" {
            "https://api.push.apple.com"
        } else {
            "https://api.sandbox.push.apple.com"
        }
    }
}

#[async_trait]
impl Transport for ApnsTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("push_apns")
    }

    fn display_name(&self) -> &str {
        "Push (APNs)"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField {
                    name: "title".into(),
                    field_type: ContentFieldType::Text,
                    required: true,
                    description: "Alert title".into(),
                },
                ContentField {
                    name: "body".into(),
                    field_type: ContentFieldType::Text,
                    required: true,
                    description: "Alert body".into(),
                },
                ContentField {
                    name: "badge".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "Badge count (parse as integer)".into(),
                },
                ContentField {
                    name: "sound".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "Sound name, default \"default\"".into(),
                },
                ContentField {
                    name: "data".into(),
                    field_type: ContentFieldType::Json,
                    required: false,
                    description: "Custom payload".into(),
                },
                ContentField {
                    name: "category".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "Notification category for actions".into(),
                },
            ],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![
                CredentialField {
                    name: "team_id".into(),
                    required: true,
                    secret: false,
                    description: "Apple Developer Team ID".into(),
                },
                CredentialField {
                    name: "key_id".into(),
                    required: true,
                    secret: false,
                    description: "Key ID from .p8 file".into(),
                },
                CredentialField {
                    name: "private_key".into(),
                    required: true,
                    secret: true,
                    description: ".p8 private key contents".into(),
                },
                CredentialField {
                    name: "environment".into(),
                    required: true,
                    secret: false,
                    description: "\"production\" or \"sandbox\"".into(),
                },
            ],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        // Extract credentials
        let team_id = message
            .credentials
            .get("team_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing team_id credential".into()))?;

        let key_id = message
            .credentials
            .get("key_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing key_id credential".into()))?;

        let private_key = message
            .credentials
            .get("private_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing private_key credential".into()))?;

        let environment = message
            .credentials
            .get("environment")
            .and_then(|v| v.as_str())
            .unwrap_or("sandbox");

        // Extract content fields
        let title = message
            .content
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing 'title' in content".into()))?;

        let body = message
            .content
            .get("body")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing 'body' in content".into()))?;

        let device_token = &message.recipient_contact;

        // Build the APNs alert
        let alert = serde_json::json!({
            "title": title,
            "body": body,
        });

        // Build the aps payload
        let sound = message
            .content
            .get("sound")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let mut aps = serde_json::json!({
            "alert": alert,
            "sound": sound,
        });

        if let Some(badge_str) = message.content.get("badge").and_then(|v| v.as_str()) {
            if let Ok(badge_num) = badge_str.parse::<u32>() {
                aps["badge"] = serde_json::Value::Number(badge_num.into());
            }
        }

        if let Some(category) = message.content.get("category").and_then(|v| v.as_str()) {
            aps["category"] = serde_json::Value::String(category.to_string());
        }

        let mut payload = serde_json::json!({
            "aps": aps,
        });

        // Merge custom data into the top-level payload
        if let Some(data) = message.content.get("data") {
            if let Some(obj) = data.as_object() {
                for (k, v) in obj {
                    payload[k] = v.clone();
                }
            }
        }

        // Generate auth token
        let token = self.generate_token(team_id, key_id, private_key)?;

        // Send the notification
        let endpoint = Self::endpoint(environment);
        let url = format!("{}/3/device/{}", endpoint, device_token);

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .header("apns-push-type", "alert")
            .json(&payload)
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("APNs HTTP error: {e}")))?;

        let status = resp.status();

        if status.is_success() {
            // APNs returns the apns-id header as the message identifier
            let apns_id = resp
                .headers()
                .get("apns-id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            Ok(DeliveryResult::Delivered {
                provider_message_id: apns_id,
            })
        } else {
            let status_code = status.as_u16();
            let error_body = resp.text().await.unwrap_or_default();

            let reason = serde_json::from_str::<ApnsErrorResponse>(&error_body)
                .ok()
                .and_then(|e| e.reason);

            let error_message = reason
                .as_deref()
                .unwrap_or(&format!("APNs API error (HTTP {status_code}): {error_body}"))
                .to_string();

            // Determine retryability
            let retryable = if let Some(ref r) = reason {
                !NON_RETRYABLE_REASONS.contains(&r.as_str())
                    && (status_code == 429 || status.is_server_error())
            } else {
                status_code == 429 || status.is_server_error()
            };

            tracing::warn!(
                device_token = %device_token,
                error = %error_message,
                status = status_code,
                retryable = retryable,
                "APNs delivery failed"
            );

            Ok(DeliveryResult::Failed {
                error: error_message,
                retryable,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apns_channel_id() {
        let transport = ApnsTransport::new();
        assert_eq!(transport.channel_id(), ChannelId::new("push_apns"));
        assert_eq!(transport.display_name(), "Push (APNs)");
    }

    #[test]
    fn apns_content_schema() {
        let transport = ApnsTransport::new();
        let content = transport.content_schema();
        assert_eq!(content.fields.len(), 6);

        assert_eq!(content.fields[0].name, "title");
        assert!(content.fields[0].required);

        assert_eq!(content.fields[1].name, "body");
        assert!(content.fields[1].required);

        assert_eq!(content.fields[2].name, "badge");
        assert!(!content.fields[2].required);

        assert_eq!(content.fields[3].name, "sound");
        assert!(!content.fields[3].required);

        assert_eq!(content.fields[4].name, "data");
        assert!(!content.fields[4].required);

        assert_eq!(content.fields[5].name, "category");
        assert!(!content.fields[5].required);
    }

    #[test]
    fn apns_credential_schema() {
        let transport = ApnsTransport::new();
        let creds = transport.credential_schema();
        assert_eq!(creds.fields.len(), 4);

        assert_eq!(creds.fields[0].name, "team_id");
        assert!(creds.fields[0].required);
        assert!(!creds.fields[0].secret);

        assert_eq!(creds.fields[1].name, "key_id");
        assert!(creds.fields[1].required);
        assert!(!creds.fields[1].secret);

        assert_eq!(creds.fields[2].name, "private_key");
        assert!(creds.fields[2].required);
        assert!(creds.fields[2].secret);

        assert_eq!(creds.fields[3].name, "environment");
        assert!(creds.fields[3].required);
        assert!(!creds.fields[3].secret);
    }

    #[tokio::test]
    async fn apns_missing_credentials() {
        let transport = ApnsTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("push_apns"),
            recipient_contact: "abc123devicetoken".into(),
            content: serde_json::json!({"title": "Hello", "body": "World"}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }
}
