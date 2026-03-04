use async_trait::async_trait;
use lettre::message::{MultiPart, SinglePart, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

use crate::channel::ChannelId;
use crate::error::CoreError;

use super::{
    ContentField, ContentFieldType, ContentSchema, CredentialField, CredentialSchema,
    DeliveryResult, RenderedMessage, Transport,
};

/// Email transport using SMTP via lettre.
pub struct EmailTransport;

#[async_trait]
impl Transport for EmailTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("email")
    }

    fn display_name(&self) -> &str {
        "Email (SMTP)"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField {
                    name: "subject".into(),
                    field_type: ContentFieldType::Text,
                    required: true,
                    description: "Email subject line".into(),
                },
                ContentField {
                    name: "text".into(),
                    field_type: ContentFieldType::Text,
                    required: true,
                    description: "Plain text body".into(),
                },
                ContentField {
                    name: "html".into(),
                    field_type: ContentFieldType::Html,
                    required: false,
                    description: "HTML body (optional, sent as multipart/alternative)".into(),
                },
            ],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![
                CredentialField {
                    name: "smtp_host".into(),
                    required: true,
                    secret: false,
                    description: "SMTP server hostname".into(),
                },
                CredentialField {
                    name: "smtp_port".into(),
                    required: false,
                    secret: false,
                    description: "SMTP server port (default: 587)".into(),
                },
                CredentialField {
                    name: "smtp_username".into(),
                    required: true,
                    secret: false,
                    description: "SMTP authentication username".into(),
                },
                CredentialField {
                    name: "smtp_password".into(),
                    required: true,
                    secret: true,
                    description: "SMTP authentication password".into(),
                },
                CredentialField {
                    name: "from_address".into(),
                    required: true,
                    secret: false,
                    description: "Sender email address".into(),
                },
                CredentialField {
                    name: "from_name".into(),
                    required: false,
                    secret: false,
                    description: "Sender display name".into(),
                },
            ],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        // Extract credentials
        let creds = &message.credentials;
        let smtp_host = creds
            .get("smtp_host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing smtp_host credential".into()))?;
        let smtp_port = creds
            .get("smtp_port")
            .and_then(|v| v.as_u64())
            .unwrap_or(587) as u16;
        let smtp_username = creds
            .get("smtp_username")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing smtp_username credential".into()))?;
        let smtp_password = creds
            .get("smtp_password")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing smtp_password credential".into()))?;
        let from_address = creds
            .get("from_address")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::InvalidConfig("Missing from_address credential".into()))?;
        let from_name = creds.get("from_name").and_then(|v| v.as_str());

        // Extract content
        let subject = message
            .content
            .get("subject")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing 'subject' in content".into()))?;
        let text = message
            .content
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing 'text' in content".into()))?;
        let html = message.content.get("html").and_then(|v| v.as_str());

        // Build From address
        let from = if let Some(name) = from_name {
            format!("{name} <{from_address}>")
        } else {
            from_address.to_string()
        };

        // Build email message
        let mut email_builder = Message::builder()
            .from(from.parse().map_err(|e| {
                CoreError::Transport(format!("Invalid from address '{from}': {e}"))
            })?)
            .to(message.recipient_contact.parse().map_err(|e| {
                CoreError::Transport(format!(
                    "Invalid recipient address '{}': {e}",
                    message.recipient_contact
                ))
            })?)
            .subject(subject);

        // Add List-Unsubscribe header placeholder for marketing emails
        email_builder = email_builder.header(lettre::message::header::ContentTransferEncoding::EightBit);

        let email = if let Some(html_body) = html {
            email_builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_PLAIN)
                                .body(text.to_string()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(ContentType::TEXT_HTML)
                                .body(html_body.to_string()),
                        ),
                )
                .map_err(|e| CoreError::Transport(format!("Failed to build email: {e}")))?
        } else {
            email_builder
                .body(text.to_string())
                .map_err(|e| CoreError::Transport(format!("Failed to build email: {e}")))?
        };

        // Build SMTP transport
        let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host)
            .map_err(|e| CoreError::Transport(format!("SMTP relay error: {e}")))?
            .port(smtp_port)
            .credentials(Credentials::new(
                smtp_username.to_string(),
                smtp_password.to_string(),
            ))
            .build();

        // Send
        match mailer.send(email).await {
            Ok(response) => {
                tracing::info!(
                    recipient = %message.recipient_contact,
                    "Email sent successfully"
                );
                Ok(DeliveryResult::Delivered {
                    provider_message_id: Some(response.message().collect::<Vec<_>>().join(" ")),
                })
            }
            Err(e) => {
                let error_str = e.to_string();
                let retryable = e.is_transient();
                tracing::warn!(
                    recipient = %message.recipient_contact,
                    error = %error_str,
                    retryable = retryable,
                    "Email delivery failed"
                );
                Ok(DeliveryResult::Failed {
                    error: error_str,
                    retryable,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_channel_id() {
        let transport = EmailTransport;
        assert_eq!(transport.channel_id(), ChannelId::new("email"));
        assert_eq!(transport.display_name(), "Email (SMTP)");
    }

    #[test]
    fn email_content_schema() {
        let transport = EmailTransport;
        let schema = transport.content_schema();
        assert_eq!(schema.fields.len(), 3);

        let subject = &schema.fields[0];
        assert_eq!(subject.name, "subject");
        assert!(subject.required);

        let text = &schema.fields[1];
        assert_eq!(text.name, "text");
        assert!(text.required);

        let html = &schema.fields[2];
        assert_eq!(html.name, "html");
        assert!(!html.required);
    }

    #[test]
    fn email_credential_schema() {
        let transport = EmailTransport;
        let schema = transport.credential_schema();

        let required_fields: Vec<&str> = schema
            .fields
            .iter()
            .filter(|f| f.required)
            .map(|f| f.name.as_str())
            .collect();
        assert!(required_fields.contains(&"smtp_host"));
        assert!(required_fields.contains(&"smtp_username"));
        assert!(required_fields.contains(&"smtp_password"));
        assert!(required_fields.contains(&"from_address"));

        let secret_fields: Vec<&str> = schema
            .fields
            .iter()
            .filter(|f| f.secret)
            .map(|f| f.name.as_str())
            .collect();
        assert!(secret_fields.contains(&"smtp_password"));
        assert!(!secret_fields.contains(&"smtp_host"));
    }
}
