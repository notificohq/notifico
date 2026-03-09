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

/// FCM HTTP v1 push notification transport.
///
/// Sends push notifications via the Firebase Cloud Messaging HTTP v1 API.
/// - recipient_contact: FCM device registration token
/// - content: `title` (required), `body` (required), `image_url` (optional),
///   `data` (optional JSON), `click_action` (optional)
/// - credentials: `service_account_json` (required, secret)
pub struct FcmTransport {
    client: Client,
}

impl FcmTransport {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl Default for FcmTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Minimal representation of a Google service account JSON key file.
#[derive(Deserialize)]
struct ServiceAccountKey {
    project_id: String,
    private_key: String,
    client_email: String,
    token_uri: String,
}

/// Response from the Google OAuth2 token endpoint.
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Response from the FCM v1 send endpoint.
#[derive(Deserialize)]
struct FcmResponse {
    name: Option<String>,
}

/// Error response from FCM.
#[derive(Deserialize)]
struct FcmErrorResponse {
    error: Option<FcmErrorDetail>,
}

#[derive(Deserialize)]
struct FcmErrorDetail {
    message: Option<String>,
}

impl FcmTransport {
    /// Generate a signed JWT for the Google OAuth2 token exchange.
    fn generate_jwt(
        &self,
        client_email: &str,
        private_key: &str,
        token_uri: &str,
    ) -> Result<String, CoreError> {
        let now = Utc::now().timestamp();
        let claims = serde_json::json!({
            "iss": client_email,
            "scope": "https://www.googleapis.com/auth/firebase.messaging",
            "aud": token_uri,
            "iat": now,
            "exp": now + 3600,
        });

        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(private_key.as_bytes())
            .map_err(|e| CoreError::InvalidConfig(format!("Invalid RSA private key: {e}")))?;

        jsonwebtoken::encode(&header, &claims, &key)
            .map_err(|e| CoreError::InvalidConfig(format!("JWT encoding error: {e}")))
    }

    /// Exchange a signed JWT for an access token at the token URI.
    async fn get_access_token(
        &self,
        jwt: &str,
        token_uri: &str,
    ) -> Result<String, CoreError> {
        let resp = self
            .client
            .post(token_uri)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", jwt),
            ])
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("FCM token exchange HTTP error: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(CoreError::Transport(format!(
                "FCM token exchange failed (HTTP {status}): {body}"
            )));
        }

        let token_resp: TokenResponse = resp
            .json()
            .await
            .map_err(|e| CoreError::Transport(format!("FCM token response parse error: {e}")))?;

        Ok(token_resp.access_token)
    }
}

#[async_trait]
impl Transport for FcmTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("push_fcm")
    }

    fn display_name(&self) -> &str {
        "Push (FCM)"
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
                    name: "image_url".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "Image URL".into(),
                },
                ContentField {
                    name: "data".into(),
                    field_type: ContentFieldType::Json,
                    required: false,
                    description: "Custom key-value payload".into(),
                },
                ContentField {
                    name: "click_action".into(),
                    field_type: ContentFieldType::Text,
                    required: false,
                    description: "Intent/URL on tap".into(),
                },
            ],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![CredentialField {
                name: "service_account_json".into(),
                required: true,
                secret: true,
                description: "Google service account JSON".into(),
            }],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        // Extract credentials
        let sa_json_str = message
            .credentials
            .get("service_account_json")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                CoreError::InvalidConfig("Missing service_account_json credential".into())
            })?;

        let sa_key: ServiceAccountKey = serde_json::from_str(sa_json_str).map_err(|e| {
            CoreError::InvalidConfig(format!("Invalid service account JSON: {e}"))
        })?;

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

        // Build notification object
        let mut notification = serde_json::json!({
            "title": title,
            "body": body,
        });

        if let Some(image_url) = message.content.get("image_url").and_then(|v| v.as_str()) {
            notification["image"] = serde_json::Value::String(image_url.to_string());
        }

        // Build message payload
        let mut fcm_message = serde_json::json!({
            "token": device_token,
            "notification": notification,
        });

        // Add optional data payload
        if let Some(data) = message.content.get("data") {
            if !data.is_null() {
                fcm_message["data"] = data.clone();
            }
        }

        // Add click_action via webpush.fcm_options.link
        if let Some(click_action) = message.content.get("click_action").and_then(|v| v.as_str()) {
            fcm_message["webpush"] = serde_json::json!({
                "fcm_options": {
                    "link": click_action,
                }
            });
        }

        let request_body = serde_json::json!({
            "message": fcm_message,
        });

        // Authenticate
        let jwt = self.generate_jwt(
            &sa_key.client_email,
            &sa_key.private_key,
            &sa_key.token_uri,
        )?;

        let access_token = self.get_access_token(&jwt, &sa_key.token_uri).await?;

        // Send the message
        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            sa_key.project_id
        );

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("FCM HTTP error: {e}")))?;

        let status = resp.status();

        if status.is_success() {
            let fcm_resp: FcmResponse = resp
                .json()
                .await
                .map_err(|e| CoreError::Transport(format!("FCM response parse error: {e}")))?;

            Ok(DeliveryResult::Delivered {
                provider_message_id: fcm_resp.name,
            })
        } else {
            let status_code = status.as_u16();
            let error_body = resp.text().await.unwrap_or_default();

            let error_message = serde_json::from_str::<FcmErrorResponse>(&error_body)
                .ok()
                .and_then(|e| e.error)
                .and_then(|e| e.message)
                .unwrap_or_else(|| format!("FCM API error (HTTP {status_code}): {error_body}"));

            // 429 and 5xx are retryable; 404 (invalid token) is not
            let retryable = status_code == 429 || status.is_server_error();

            tracing::warn!(
                device_token = %device_token,
                error = %error_message,
                status = status_code,
                retryable = retryable,
                "FCM delivery failed"
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
    fn fcm_channel_id() {
        let transport = FcmTransport::new();
        assert_eq!(transport.channel_id(), ChannelId::new("push_fcm"));
        assert_eq!(transport.display_name(), "Push (FCM)");
    }

    #[test]
    fn fcm_content_schema() {
        let transport = FcmTransport::new();
        let content = transport.content_schema();
        assert_eq!(content.fields.len(), 5);

        assert_eq!(content.fields[0].name, "title");
        assert!(content.fields[0].required);

        assert_eq!(content.fields[1].name, "body");
        assert!(content.fields[1].required);

        assert_eq!(content.fields[2].name, "image_url");
        assert!(!content.fields[2].required);

        assert_eq!(content.fields[3].name, "data");
        assert!(!content.fields[3].required);

        assert_eq!(content.fields[4].name, "click_action");
        assert!(!content.fields[4].required);
    }

    #[test]
    fn fcm_credential_schema() {
        let transport = FcmTransport::new();
        let creds = transport.credential_schema();
        assert_eq!(creds.fields.len(), 1);
        assert_eq!(creds.fields[0].name, "service_account_json");
        assert!(creds.fields[0].required);
        assert!(creds.fields[0].secret);
    }

    #[tokio::test]
    async fn fcm_missing_credentials() {
        let transport = FcmTransport::new();
        let message = RenderedMessage {
            channel: ChannelId::new("push_fcm"),
            recipient_contact: "device-token-123".into(),
            content: serde_json::json!({"title": "Hello", "body": "World"}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };

        let result = transport.send(&message).await;
        assert!(result.is_err());
    }
}
