use async_trait::async_trait;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;

use notifico_core::channel::ChannelId;
use notifico_core::error::CoreError;
use notifico_core::transport::{
    ContentField, ContentFieldType, ContentSchema, CredentialField, CredentialSchema,
    DeliveryResult, RenderedMessage, Transport,
};

type HmacSha256 = Hmac<Sha256>;

/// Generic webhook transport that POSTs JSON to a configurable URL.
///
/// - recipient_contact: ignored (URL comes from credentials)
/// - content: `body` (JSON, required), `method` (optional: GET/POST/PUT, default POST)
/// - credentials: `url` (required), `headers` (optional JSON object), `secret` (optional HMAC key)
pub struct WebhookTransport {
    client: Client,
}

impl WebhookTransport {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for WebhookTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for WebhookTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("webhook")
    }

    fn display_name(&self) -> &str {
        "Webhook (HTTP)"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField {
                    name: "body".into(),
                    field_type: ContentFieldType::Json,
                    required: true,
                    description: "JSON payload to send".into(),
                },
                ContentField {
                    name: "method".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "HTTP method: GET, POST, PUT (default: POST)".into(),
                },
            ],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![
                CredentialField {
                    name: "url".into(),
                    required: true,
                    secret: false,
                    description: "Webhook endpoint URL".into(),
                },
                CredentialField {
                    name: "headers".into(),
                    required: false,
                    secret: false,
                    description: "Additional HTTP headers as JSON object".into(),
                },
                CredentialField {
                    name: "secret".into(),
                    required: false,
                    secret: true,
                    description: "HMAC-SHA256 signing secret for X-Notifico-Signature header".into(),
                },
            ],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        let creds = &message.credentials;

        let url = creds
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing url credential".into()))?;

        let body = message
            .content
            .get("body")
            .ok_or_else(|| CoreError::Transport("Missing 'body' in content".into()))?;

        let method = message
            .content
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("POST");

        let body_bytes = serde_json::to_vec(body)
            .map_err(|e| CoreError::Transport(format!("Failed to serialize body: {e}")))?;

        let req_builder = match method.to_uppercase().as_str() {
            "GET" => self.client.get(url),
            "PUT" => self.client.put(url),
            _ => self.client.post(url),
        };

        let mut req_builder = req_builder
            .header("content-type", "application/json")
            .body(body_bytes.clone());

        // Add custom headers from credentials
        if let Some(headers) = creds.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in headers {
                if let Some(val) = value.as_str() {
                    req_builder = req_builder.header(key, val);
                }
            }
        }

        // Add HMAC signature if secret is provided
        if let Some(secret) = creds.get("secret").and_then(|v| v.as_str()) {
            let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                .map_err(|e| CoreError::Transport(format!("Invalid HMAC key: {e}")))?;
            mac.update(&body_bytes);
            let signature = hex::encode(mac.finalize().into_bytes());
            req_builder = req_builder.header("X-Notifico-Signature", format!("sha256={signature}"));
        }

        let resp = req_builder
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("Webhook HTTP error: {e}")))?;

        let status = resp.status();

        if status.is_success() {
            Ok(DeliveryResult::Delivered {
                provider_message_id: None,
            })
        } else {
            let error = format!("Webhook returned HTTP {}", status.as_u16());
            let retryable = status.is_server_error() || status.as_u16() == 429;
            tracing::warn!(
                url = %url,
                status = %status.as_u16(),
                retryable = retryable,
                "Webhook delivery failed"
            );
            Ok(DeliveryResult::Failed { error, retryable })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_schema() {
        let transport = WebhookTransport::new();
        assert_eq!(transport.channel_id(), ChannelId::new("webhook"));
        assert_eq!(transport.display_name(), "Webhook (HTTP)");

        let content = transport.content_schema();
        assert_eq!(content.fields.len(), 2);
        assert_eq!(content.fields[0].name, "body");
        assert!(content.fields[0].required);
        assert_eq!(content.fields[1].name, "method");
        assert!(!content.fields[1].required);

        let creds = transport.credential_schema();
        assert_eq!(creds.fields.len(), 3);
        assert_eq!(creds.fields[0].name, "url");
        assert!(creds.fields[0].required);
        assert_eq!(creds.fields[2].name, "secret");
        assert!(creds.fields[2].secret);
    }

    #[tokio::test]
    async fn webhook_missing_url() {
        let transport = WebhookTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("webhook"),
            recipient_contact: "ignored".into(),
            content: serde_json::json!({"body": {"event": "test"}}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }

    #[test]
    fn hmac_signature_computation() {
        // Verify HMAC-SHA256 produces expected output
        let secret = "test-secret";
        let body = b"{\"hello\":\"world\"}";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let signature = hex::encode(mac.finalize().into_bytes());

        // Signature should be a 64-char hex string
        assert_eq!(signature.len(), 64);
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
