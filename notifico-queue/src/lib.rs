use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A delivery task to be enqueued and processed by workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryTask {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_name: String,
    pub recipient_id: Uuid,
    pub channel: String,
    pub rendered_body: Value,
    pub contact_value: String,
    pub idempotency_key: Option<String>,
    pub attempt: u32,
    pub max_attempts: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_task_serialization_roundtrip() {
        let task = DeliveryTask {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            event_name: "order.confirmed".into(),
            recipient_id: Uuid::now_v7(),
            channel: "email".into(),
            rendered_body: serde_json::json!({"subject": "Hi", "text": "Hello"}),
            contact_value: "user@example.com".into(),
            idempotency_key: Some("key-123".into()),
            attempt: 0,
            max_attempts: 5,
        };

        let json = serde_json::to_string(&task).unwrap();
        let deserialized: DeliveryTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_name, "order.confirmed");
        assert_eq!(deserialized.channel, "email");
    }

    #[test]
    fn delivery_task_without_idempotency_key() {
        let task = DeliveryTask {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            event_name: "user.signup".into(),
            recipient_id: Uuid::now_v7(),
            channel: "sms".into(),
            rendered_body: serde_json::json!({"text": "Welcome!"}),
            contact_value: "+1234567890".into(),
            idempotency_key: None,
            attempt: 0,
            max_attempts: 3,
        };

        let json = serde_json::to_string(&task).unwrap();
        let deserialized: DeliveryTask = serde_json::from_str(&json).unwrap();
        assert!(deserialized.idempotency_key.is_none());
    }

    #[test]
    fn delivery_task_attempt_tracking() {
        let mut task = DeliveryTask {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            event_name: "test".into(),
            recipient_id: Uuid::now_v7(),
            channel: "email".into(),
            rendered_body: serde_json::json!({}),
            contact_value: "test@test.com".into(),
            idempotency_key: None,
            attempt: 0,
            max_attempts: 5,
        };

        task.attempt += 1;
        assert_eq!(task.attempt, 1);
        assert!(task.attempt < task.max_attempts);
    }
}
