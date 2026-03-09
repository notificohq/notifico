use std::sync::Arc;

use sea_orm::DatabaseConnection;
use uuid::Uuid;

use notifico_core::channel::ChannelId;
use notifico_core::middleware::MiddlewareRegistry;
use notifico_core::registry::TransportRegistry;
use notifico_core::transport::{DeliveryResult, RenderedMessage};
use notifico_db::repo;
use notifico_queue::DeliveryTask;

use crate::AppState;

/// Run the worker loop: poll queue, claim tasks, process, update status.
/// Shuts down gracefully on SIGTERM or SIGINT (Ctrl+C).
pub async fn run_worker_loop(state: Arc<AppState>) {
    let poll_interval = std::time::Duration::from_secs(2);

    tracing::info!("Worker loop started");

    let mut shutdown = std::pin::pin!(shutdown_signal());

    loop {
        // Check for shutdown between batches
        tokio::select! {
            _ = &mut shutdown => {
                tracing::info!("Worker shutting down gracefully");
                return;
            }
            tasks_result = repo::queue::claim_pending(&state.db, 10) => {
                let tasks = match tasks_result {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to claim tasks");
                        tokio::time::sleep(poll_interval).await;
                        continue;
                    }
                };

                if tasks.is_empty() {
                    tokio::time::sleep(poll_interval).await;
                    continue;
                }

                tracing::info!(count = tasks.len(), "Claimed delivery tasks");

                // Process the entire batch before checking shutdown again
                for task_row in &tasks {
                    let delivery_task = task_row_to_delivery_task(task_row);

                    match process_delivery(&delivery_task, &state.registry, &state.middleware_registry, &state.db, state.encryption_key.as_ref()).await {
                        Ok(()) => {
                            if let Err(e) =
                                repo::queue::mark_completed(&state.db, task_row.id).await
                            {
                                tracing::error!(
                                    task_id = %task_row.id, error = %e,
                                    "Failed to mark completed"
                                );
                            }
                        }
                        Err(reason) => {
                            if let Err(e) = repo::queue::mark_failed(
                                &state.db,
                                task_row.id,
                                &reason,
                                true,
                                task_row.attempt,
                                task_row.max_attempts,
                            )
                            .await
                            {
                                tracing::error!(
                                    task_id = %task_row.id, error = %e,
                                    "Failed to mark failed"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to register SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
    }
}

fn task_row_to_delivery_task(row: &repo::queue::TaskRow) -> DeliveryTask {
    DeliveryTask {
        id: row.id,
        project_id: row.project_id,
        event_name: row.event_name.clone(),
        recipient_id: row.recipient_id,
        channel: row.channel.clone(),
        rendered_body: row.rendered_body.clone(),
        contact_value: row.contact_value.clone(),
        idempotency_key: row.idempotency_key.clone(),
        rule_id: row.rule_id,
        attempt: row.attempt as u32,
        max_attempts: row.max_attempts as u32,
    }
}

/// Process a single delivery task.
#[tracing::instrument(
    name = "process_delivery",
    skip(task, registry, middleware_registry, db, encryption_key),
    fields(task_id = %task.id, channel = %task.channel, recipient_id = %task.recipient_id)
)]
pub async fn process_delivery(
    task: &DeliveryTask,
    registry: &TransportRegistry,
    middleware_registry: &MiddlewareRegistry,
    db: &DatabaseConnection,
    encryption_key: Option<&[u8; 32]>,
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

    // Resolve credentials for this transport
    let credentials = if let Some(key) = encryption_key {
        match repo::credential::find_credential(db, task.project_id, &task.channel, key).await {
            Ok(Some(cred)) => cred.data,
            Ok(None) => {
                // Check if transport requires credentials
                let schema = transport.credential_schema();
                if schema.fields.iter().any(|f| f.required) {
                    return Err(format!(
                        "No credentials configured for channel '{}' in project {}",
                        task.channel, task.project_id
                    ));
                }
                serde_json::json!({})
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to resolve credentials, proceeding without");
                serde_json::json!({})
            }
        }
    } else {
        serde_json::json!({})
    };

    // Fetch middleware config if the task has a rule_id
    let mw_entries = if let Some(rule_id) = task.rule_id {
        match repo::middleware::list_by_rule(db, rule_id).await {
            Ok(entries) => entries,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to fetch middleware config, proceeding without");
                vec![]
            }
        }
    } else {
        vec![]
    };

    // Build RenderedMessage from task
    let mut message = RenderedMessage {
        channel: channel_id,
        recipient_contact: task.contact_value.clone(),
        content: task.rendered_body.clone(),
        credentials,
        attachments: vec![],
    };

    // Run pre-send middleware
    for entry in &mw_entries {
        if let Some(mw) = middleware_registry.get(&entry.middleware_name) {
            let config = match entry.config_value() {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        middleware = %entry.middleware_name,
                        error = %e,
                        "Invalid middleware config, skipping"
                    );
                    continue;
                }
            };
            if let Err(e) = mw.pre_send(&mut message, &config).await {
                tracing::warn!(
                    middleware = %entry.middleware_name,
                    error = %e,
                    "pre_send middleware error, skipping"
                );
            }
        } else {
            tracing::warn!(
                middleware = %entry.middleware_name,
                "Middleware not found in registry, skipping"
            );
        }
    }

    // Send via transport
    let result = transport.send(&message).await;

    // Run post-send middleware
    if let Ok(ref delivery_result) = result {
        for entry in &mw_entries {
            if let Some(mw) = middleware_registry.get(&entry.middleware_name) {
                let config = match entry.config_value() {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!(
                            middleware = %entry.middleware_name,
                            error = %e,
                            "Invalid middleware config, skipping"
                        );
                        continue;
                    }
                };
                if let Err(e) = mw.post_send(&message, delivery_result, &config).await {
                    tracing::warn!(
                        middleware = %entry.middleware_name,
                        error = %e,
                        "post_send middleware error"
                    );
                }
            }
        }
    }

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
    if let Err(e) = repo::delivery_log::insert_log(
        db,
        Uuid::now_v7(),
        task.project_id,
        &task.event_name,
        task.recipient_id,
        &task.channel,
        status,
        error_message,
        (task.attempt + 1) as i32,
    )
    .await
    {
        tracing::error!(error = %e, "Failed to log delivery result");
    }
}
