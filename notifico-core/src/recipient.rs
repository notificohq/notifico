use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::channel::ChannelId;

/// A recipient in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    pub id: Uuid,
    pub project_id: Uuid,
    pub external_id: String,
    pub locale: String,
    pub timezone: String,
    pub metadata: serde_json::Value,
}

/// A contact method for a recipient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipientContact {
    pub id: Uuid,
    pub recipient_id: Uuid,
    pub channel: ChannelId,
    pub value: String,
    pub verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipient_serialization_roundtrip() {
        let recipient = Recipient {
            id: Uuid::nil(),
            project_id: Uuid::nil(),
            external_id: "user-123".to_string(),
            locale: "ru".to_string(),
            timezone: "Europe/Moscow".to_string(),
            metadata: serde_json::json!({"tier": "premium"}),
        };
        let json = serde_json::to_string(&recipient).unwrap();
        let deserialized: Recipient = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.external_id, "user-123");
        assert_eq!(deserialized.locale, "ru");
    }

    #[test]
    fn recipient_contact_serialization() {
        let contact = RecipientContact {
            id: Uuid::nil(),
            recipient_id: Uuid::nil(),
            channel: ChannelId::new("email"),
            value: "test@example.com".to_string(),
            verified: true,
        };
        let json = serde_json::to_string(&contact).unwrap();
        assert!(json.contains("\"email\""));
        assert!(json.contains("test@example.com"));
    }
}
