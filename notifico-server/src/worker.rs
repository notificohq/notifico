use sea_orm::{ConnectionTrait, DatabaseConnection};
use uuid::Uuid;

use notifico_core::channel::ChannelId;
use notifico_core::registry::TransportRegistry;
use notifico_core::transport::{DeliveryResult, RenderedMessage};
use notifico_queue::DeliveryTask;

/// Process a single delivery task.
pub async fn process_delivery(
    task: &DeliveryTask,
    registry: &TransportRegistry,
    db: &DatabaseConnection,
) -> Result<(), String> {
    tracing::info!(
        task_id = %task.id,
        channel = %task.channel,
        recipient = %task.recipient_id,
        attempt = task.attempt,
        "Processing delivery task"
    );

    // Look up transport by channel
    let channel_id = ChannelId::new(&task.channel);
    let transport = registry
        .get(&channel_id)
        .ok_or_else(|| format!("Transport not found for channel: {}", task.channel))?;

    // Build RenderedMessage from task
    let message = RenderedMessage {
        channel: channel_id,
        recipient_contact: task.contact_value.clone(),
        content: task.rendered_body.clone(),
        credentials: serde_json::json!({}), // Credentials will be resolved in a later phase
        attachments: vec![],
    };

    // Send via transport
    let result = transport.send(&message).await;

    match result {
        Ok(delivery_result) => match delivery_result {
            DeliveryResult::Delivered {
                provider_message_id,
            } => {
                log_delivery(db, task, "delivered", None).await;
                tracing::info!(
                    task_id = %task.id,
                    provider_id = ?provider_message_id,
                    "Delivery successful"
                );
                Ok(())
            }
            DeliveryResult::Failed { error, retryable } => {
                let status = if retryable && task.attempt < task.max_attempts {
                    "queued"
                } else {
                    "failed"
                };
                log_delivery(db, task, status, Some(&error)).await;

                if retryable && task.attempt < task.max_attempts {
                    Err(format!("Retryable failure: {error}"))
                } else {
                    tracing::error!(task_id = %task.id, error = %error, "Delivery permanently failed");
                    Ok(())
                }
            }
        },
        Err(e) => {
            let reason = e.to_string();
            log_delivery(db, task, "failed", Some(&reason)).await;
            tracing::error!(task_id = %task.id, error = %reason, "Transport error");
            Err(reason)
        }
    }
}

async fn log_delivery(
    db: &DatabaseConnection,
    task: &DeliveryTask,
    status: &str,
    error_message: Option<&str>,
) {
    let id = Uuid::now_v7();
    let error_msg = error_message.unwrap_or("");
    let delivered_at = if status == "delivered" {
        "CURRENT_TIMESTAMP"
    } else {
        "NULL"
    };

    let sql = format!(
        "INSERT INTO delivery_log (id, project_id, event_name, recipient_id, channel, status, error_message, attempts, delivered_at) \
         VALUES ('{id}', '{}', '{}', '{}', '{}', '{status}', '{error_msg}', {}, {delivered_at})",
        task.project_id, task.event_name, task.recipient_id, task.channel, task.attempt + 1,
    );

    if let Err(e) = db.execute_unprepared(&sql).await {
        tracing::error!(error = %e, "Failed to log delivery result");
    }
}
