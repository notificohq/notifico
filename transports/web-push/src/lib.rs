use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use notifico_core::channel::ChannelId;
use notifico_core::error::CoreError;
use notifico_core::transport::{
    ContentField, ContentFieldType, ContentSchema, CredentialField, CredentialSchema,
    DeliveryResult, RenderedMessage, Transport,
};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use web_push_native::jwt_simple::algorithms::ES256KeyPair;
use web_push_native::p256::PublicKey;
use web_push_native::{Auth, WebPushBuilder};

/// Web Push notification transport (RFC 8030 + VAPID).
///
/// Sends push notifications using the Web Push protocol with VAPID
/// authentication.
/// - recipient_contact: Push subscription JSON string
///   `{"endpoint":"https://...","keys":{"p256dh":"...","auth":"..."}}`
/// - content: `title` (required), `body` (required), `icon` (optional),
///   `url` (optional), `badge` (optional), `data` (optional JSON)
/// - credentials: `vapid_private_key` (required, secret),
///   `vapid_public_key` (required), `subject` (required)
pub struct WebPushTransport {
    client: Client,
}

impl WebPushTransport {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for WebPushTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Push subscription as received from the browser's PushManager.subscribe().
#[derive(Deserialize)]
struct PushSubscription {
    endpoint: String,
    keys: PushSubscriptionKeys,
}

#[derive(Deserialize)]
struct PushSubscriptionKeys {
    p256dh: String,
    auth: String,
}

#[async_trait]
impl Transport for WebPushTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("push_web")
    }

    fn display_name(&self) -> &str {
        "Push (Web)"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField {
                    name: "title".into(),
                    field_type: ContentFieldType::Text,
                    required: true,
                    description: "Notification title".into(),
                },
                ContentField {
                    name: "body".into(),
                    field_type: ContentFieldType::Text,
                    required: true,
                    description: "Notification body".into(),
                },
                ContentField {
                    name: "icon".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "Icon URL".into(),
                },
                ContentField {
                    name: "url".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "Click destination URL".into(),
                },
                ContentField {
                    name: "badge".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "Badge icon URL".into(),
                },
                ContentField {
                    name: "data".into(),
                    field_type: ContentFieldType::Json,
                    required: false,
                    description: "Custom payload".into(),
                },
            ],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![
                CredentialField {
                    name: "vapid_private_key".into(),
                    required: true,
                    secret: true,
                    description: "VAPID private key (base64url-encoded)".into(),
                },
                CredentialField {
                    name: "vapid_public_key".into(),
                    required: true,
                    secret: false,
                    description: "VAPID public key (base64url-encoded)".into(),
                },
                CredentialField {
                    name: "subject".into(),
                    required: true,
                    secret: false,
                    description: "Contact URI (mailto: or https://)".into(),
                },
            ],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        // Parse push subscription from the recipient contact
        let subscription: PushSubscription =
            serde_json::from_str(&message.recipient_contact).map_err(|e| {
                CoreError::Transport(format!("Invalid push subscription JSON: {e}"))
            })?;

        // Extract credentials
        let vapid_private_key = message
            .credentials
            .get("vapid_private_key")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CoreError::InvalidConfig("Missing vapid_private_key credential".into())
            })?;

        let subject = message
            .credentials
            .get("subject")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing subject credential".into()))?;

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

        // Build notification payload JSON
        let mut notification = serde_json::json!({
            "title": title,
            "body": body,
        });

        if let Some(icon) = message.content.get("icon").and_then(|v| v.as_str()) {
            notification["icon"] = serde_json::Value::String(icon.to_string());
        }

        if let Some(url) = message.content.get("url").and_then(|v| v.as_str()) {
            notification["url"] = serde_json::Value::String(url.to_string());
        }

        if let Some(badge) = message.content.get("badge").and_then(|v| v.as_str()) {
            notification["badge"] = serde_json::Value::String(badge.to_string());
        }

        if let Some(data) = message.content.get("data") {
            if !data.is_null() {
                notification["data"] = data.clone();
            }
        }

        let payload_bytes = serde_json::to_vec(&notification)
            .map_err(|e| CoreError::Transport(format!("Failed to serialize payload: {e}")))?;

        // Decode the VAPID private key and build the key pair
        let vapid_key_bytes = URL_SAFE_NO_PAD.decode(vapid_private_key).map_err(|e| {
            CoreError::InvalidConfig(format!("Invalid base64url VAPID private key: {e}"))
        })?;

        let key_pair = ES256KeyPair::from_bytes(&vapid_key_bytes).map_err(|e| {
            CoreError::InvalidConfig(format!("Invalid VAPID key pair: {e}"))
        })?;

        // Decode the subscriber's p256dh public key and auth secret
        let p256dh_bytes = URL_SAFE_NO_PAD
            .decode(&subscription.keys.p256dh)
            .map_err(|e| {
                CoreError::Transport(format!("Invalid base64url p256dh key: {e}"))
            })?;

        let ua_public = PublicKey::from_sec1_bytes(&p256dh_bytes)
            .map_err(|e| CoreError::Transport(format!("Invalid p256dh public key: {e}")))?;

        let auth_bytes = URL_SAFE_NO_PAD
            .decode(&subscription.keys.auth)
            .map_err(|e| CoreError::Transport(format!("Invalid base64url auth secret: {e}")))?;

        let ua_auth = Auth::clone_from_slice(&auth_bytes);

        // Build the encrypted web push request
        let endpoint = subscription.endpoint.parse().map_err(|e| {
            CoreError::Transport(format!("Invalid push endpoint URI: {e}"))
        })?;

        let builder = WebPushBuilder::new(endpoint, ua_public, ua_auth)
            .with_vapid(&key_pair, subject);

        let http_request = builder.build(payload_bytes).map_err(|e| {
            CoreError::Transport(format!("Web push encryption/build error: {e}"))
        })?;

        // Convert http::Request to a reqwest request and send
        let (parts, body) = http_request.into_parts();

        let mut req = self
            .client
            .post(parts.uri.to_string());

        for (name, value) in &parts.headers {
            req = req.header(name.as_str(), value.as_bytes());
        }

        let resp = req
            .body(body)
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("Web push HTTP error: {e}")))?;

        let status = resp.status();

        if status.is_success() {
            Ok(DeliveryResult::Delivered {
                provider_message_id: None,
            })
        } else {
            let status_code = status.as_u16();
            let error_body = resp.text().await.unwrap_or_default();

            let error_message =
                format!("Web push delivery failed (HTTP {status_code}): {error_body}");

            // 410 Gone = subscription expired (not retryable)
            // 429 Too Many Requests and 5xx = retryable
            let retryable = status_code != 410
                && (status_code == 429 || status.is_server_error());

            tracing::warn!(
                endpoint = %subscription.endpoint,
                error = %error_message,
                status = status_code,
                retryable = retryable,
                "Web push delivery failed"
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
    fn web_push_channel_id() {
        let transport = WebPushTransport::new();
        assert_eq!(transport.channel_id(), ChannelId::new("push_web"));
        assert_eq!(transport.display_name(), "Push (Web)");
    }

    #[test]
    fn web_push_content_schema() {
        let transport = WebPushTransport::new();
        let content = transport.content_schema();
        assert_eq!(content.fields.len(), 6);

        assert_eq!(content.fields[0].name, "title");
        assert!(content.fields[0].required);

        assert_eq!(content.fields[1].name, "body");
        assert!(content.fields[1].required);

        assert_eq!(content.fields[2].name, "icon");
        assert!(!content.fields[2].required);

        assert_eq!(content.fields[3].name, "url");
        assert!(!content.fields[3].required);

        assert_eq!(content.fields[4].name, "badge");
        assert!(!content.fields[4].required);

        assert_eq!(content.fields[5].name, "data");
        assert!(!content.fields[5].required);
    }

    #[test]
    fn web_push_credential_schema() {
        let transport = WebPushTransport::new();
        let creds = transport.credential_schema();
        assert_eq!(creds.fields.len(), 3);

        assert_eq!(creds.fields[0].name, "vapid_private_key");
        assert!(creds.fields[0].required);
        assert!(creds.fields[0].secret);

        assert_eq!(creds.fields[1].name, "vapid_public_key");
        assert!(creds.fields[1].required);
        assert!(!creds.fields[1].secret);

        assert_eq!(creds.fields[2].name, "subject");
        assert!(creds.fields[2].required);
        assert!(!creds.fields[2].secret);
    }

    #[tokio::test]
    async fn web_push_invalid_subscription() {
        let transport = WebPushTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("push_web"),
            recipient_contact: "not-valid-json".into(),
            content: serde_json::json!({"title": "Hello", "body": "World"}),
            credentials: serde_json::json!({
                "vapid_private_key": "dGVzdA",
                "vapid_public_key": "dGVzdA",
                "subject": "mailto:test@example.com"
            }),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }
}
