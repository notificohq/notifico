use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Notification category determines unsubscribe rules and delivery behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    /// Cannot unsubscribe (order confirmation, password reset)
    Transactional,
    /// Can unsubscribe, respects user schedules
    Marketing,
    /// Technical notifications, no user preferences
    System,
}

/// An ingest event sent by a client application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestEvent {
    /// Event name (e.g. "order.confirmed")
    pub event: String,
    /// Recipients to notify
    pub recipients: Vec<EventRecipient>,
    /// Template data
    pub data: serde_json::Value,
    /// Optional idempotency key
    #[serde(default)]
    pub idempotency_key: Option<String>,
}

/// Recipient within an ingest event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecipient {
    /// External ID from the client system
    pub id: String,
    /// Optional overrides for contact info
    #[serde(default)]
    pub contacts: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_category_serialization() {
        let cat = EventCategory::Transactional;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"transactional\"");

        let deserialized: EventCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, EventCategory::Transactional);
    }

    #[test]
    fn event_category_all_variants() {
        for (variant, expected) in [
            (EventCategory::Transactional, "\"transactional\""),
            (EventCategory::Marketing, "\"marketing\""),
            (EventCategory::System, "\"system\""),
        ] {
            assert_eq!(serde_json::to_string(&variant).unwrap(), expected);
        }
    }

    #[test]
    fn ingest_event_deserialization() {
        let json = r#"{
            "event": "order.confirmed",
            "recipients": [{"id": "user-123", "contacts": {"email": "test@example.com"}}],
            "data": {"order_id": 42}
        }"#;
        let event: IngestEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event, "order.confirmed");
        assert_eq!(event.recipients.len(), 1);
        assert_eq!(event.recipients[0].id, "user-123");
        assert_eq!(
            event.recipients[0].contacts.get("email").unwrap(),
            "test@example.com"
        );
        assert!(event.idempotency_key.is_none());
    }

    #[test]
    fn ingest_event_with_idempotency_key() {
        let json = r#"{
            "event": "user.signup",
            "recipients": [{"id": "u-1"}],
            "data": {},
            "idempotency_key": "abc-123"
        }"#;
        let event: IngestEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.idempotency_key.as_deref(), Some("abc-123"));
    }

    #[test]
    fn event_recipient_empty_contacts() {
        let json = r#"{"id": "user-456"}"#;
        let recipient: EventRecipient = serde_json::from_str(json).unwrap();
        assert_eq!(recipient.id, "user-456");
        assert!(recipient.contacts.is_empty());
    }
}
