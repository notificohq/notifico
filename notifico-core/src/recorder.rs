use tracing::{error, info};
use uuid::Uuid;

pub trait Recorder: Send + Sync + 'static {
    fn record_message_sent(&self, event_id: Uuid, notification_id: Uuid, message_id: Uuid);
    fn record_message_failed(
        &self,
        event_id: Uuid,
        notification_id: Uuid,
        message_id: Uuid,
        error: &str,
    );
}

pub struct BaseRecorder {}

impl BaseRecorder {
    pub fn new() -> Self {
        Self {}
    }
}

impl Recorder for BaseRecorder {
    fn record_message_sent(&self, event_id: Uuid, notification_id: Uuid, message_id: Uuid) {
        info!("Message sent: {event_id}/{notification_id}/{message_id}");
    }

    fn record_message_failed(
        &self,
        event_id: Uuid,
        notification_id: Uuid,
        message_id: Uuid,
        error: &str,
    ) {
        error!("Failed to send message: {event_id}/{notification_id}/{message_id} - {error}");
    }
}
