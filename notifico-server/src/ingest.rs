use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use uuid::Uuid;

use notifico_core::event::IngestEvent;
use notifico_core::pipeline::{PipelineInput, execute_pipeline};
use notifico_db::repo;

use crate::AppState;
use crate::auth::AuthContext;

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub accepted: usize,
    pub task_ids: Vec<Uuid>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

pub async fn handle_ingest(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(event): Json<IngestEvent>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    auth.require_scope("ingest")
        .map_err(|e| (StatusCode::FORBIDDEN, format!("{e:?}")))?;

    let project_id = auth.project_id;
    let default_locale = &state.config.project.default_locale;

    // Resolve event by name
    let event_row = repo::template::find_event_by_name(&state.db, project_id, &event.event)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Event not found: {}", event.event),
            )
        })?;

    // Get pipeline rules for this event
    let rules = repo::template::get_pipeline_rules(&state.db, event_row.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if rules.is_empty() {
        return Ok(Json(IngestResponse {
            accepted: 0,
            task_ids: vec![],
            errors: vec![format!(
                "No pipeline rules configured for event: {}",
                event.event
            )],
        }));
    }

    let mut task_ids = Vec::new();
    let mut errors = Vec::new();

    for recipient_input in &event.recipients {
        // Resolve or upsert recipient
        let recipient_id = match repo::recipient::upsert_recipient(
            &state.db,
            project_id,
            &recipient_input.id,
            None,
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                errors.push(format!(
                    "Failed to resolve recipient {}: {}",
                    recipient_input.id, e
                ));
                continue;
            }
        };

        // Store inline contacts if provided
        for (channel, value) in &recipient_input.contacts {
            if let Err(e) =
                repo::recipient::upsert_contact(&state.db, recipient_id, channel, value).await
            {
                tracing::warn!(
                    recipient = %recipient_input.id,
                    channel = %channel,
                    error = %e,
                    "Failed to upsert contact"
                );
            }
        }

        // Get recipient info for locale
        let recipient_row =
            repo::recipient::find_by_external_id(&state.db, project_id, &recipient_input.id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let recipient_locale = recipient_row
            .as_ref()
            .map(|r| r.locale.as_str())
            .unwrap_or(default_locale);

        // Get contacts from DB
        let db_contacts = repo::recipient::get_contacts(&state.db, recipient_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for rule in &rules {
            // Find contact value for this channel: inline contacts > DB contacts
            let contact_value = recipient_input
                .contacts
                .get(&rule.channel)
                .cloned()
                .or_else(|| {
                    db_contacts
                        .iter()
                        .find(|c| c.channel == rule.channel)
                        .map(|c| c.value.clone())
                });

            let contact_value = match contact_value {
                Some(v) => v,
                None => {
                    errors.push(format!(
                        "No contact for recipient {} on channel {}",
                        recipient_input.id, rule.channel
                    ));
                    continue;
                }
            };

            // Check idempotency
            if let Some(ref client_key) = event.idempotency_key {
                let idem_key = repo::idempotency::make_idempotency_key(
                    &event.event,
                    recipient_id,
                    &rule.channel,
                    Some(client_key),
                );
                match repo::idempotency::check_and_insert(&state.db, &idem_key).await {
                    Ok(true) => {
                        tracing::debug!(key = %idem_key, "Duplicate delivery skipped");
                        continue;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        tracing::error!(error = %e, "Idempotency check failed");
                        // Proceed anyway — better to deliver twice than not at all
                    }
                }
            }

            // Resolve template
            let template = match repo::template::resolve_template(
                &state.db,
                rule.template_id,
                recipient_locale,
                default_locale,
            )
            .await
            {
                Ok(Some(t)) => t,
                Ok(None) => {
                    errors.push(format!(
                        "Template not found for rule {} (template_id: {}, locale: {})",
                        rule.id, rule.template_id, recipient_locale
                    ));
                    continue;
                }
                Err(e) => {
                    errors.push(format!("Template resolution error: {}", e));
                    continue;
                }
            };

            // Execute pipeline (render template)
            let pipeline_input = PipelineInput {
                project_id,
                event_name: event.event.clone(),
                recipient_id,
                recipient_locale: recipient_locale.to_string(),
                channel: rule.channel.clone(),
                contact_value,
                template_body: template.body,
                context_data: event.data.clone(),
                idempotency_key: event.idempotency_key.clone(),
                max_attempts: 5,
            };

            match execute_pipeline(pipeline_input) {
                Ok(output) => {
                    if let Err(e) = repo::queue::enqueue(
                        &state.db,
                        output.id,
                        output.project_id,
                        &output.event_name,
                        output.recipient_id,
                        &output.channel,
                        &output.contact_value,
                        &output.rendered_body,
                        output.idempotency_key.as_deref(),
                        output.max_attempts as i32,
                    )
                    .await
                    {
                        errors.push(format!(
                            "Failed to enqueue task for recipient {} channel {}: {}",
                            recipient_input.id, rule.channel, e
                        ));
                        continue;
                    }

                    task_ids.push(output.id);
                    tracing::info!(
                        task_id = %output.id,
                        channel = %output.channel,
                        recipient = %recipient_input.id,
                        "Delivery task enqueued"
                    );
                }
                Err(e) => {
                    errors.push(format!(
                        "Pipeline error for recipient {} channel {}: {}",
                        recipient_input.id, rule.channel, e
                    ));
                }
            }
        }
    }

    let accepted = task_ids.len();
    Ok(Json(IngestResponse {
        accepted,
        task_ids,
        errors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingest_response_serialization() {
        let resp = IngestResponse {
            accepted: 2,
            task_ids: vec![Uuid::now_v7(), Uuid::now_v7()],
            errors: vec![],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["accepted"], 2);
        // errors is empty, should be skipped
        assert!(json.get("errors").is_none());
    }

    #[test]
    fn ingest_response_with_errors() {
        let resp = IngestResponse {
            accepted: 1,
            task_ids: vec![Uuid::now_v7()],
            errors: vec!["No contact for user on sms".into()],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["accepted"], 1);
        assert_eq!(json["errors"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn ingest_event_parsing() {
        let json_str = r#"{
            "event": "order.confirmed",
            "recipients": [
                {"id": "user-123", "contacts": {"email": "test@example.com"}},
                {"id": "user-456"}
            ],
            "data": {"order_id": 42, "total": "99.90"},
            "idempotency_key": "order-42-confirm"
        }"#;
        let event: IngestEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.event, "order.confirmed");
        assert_eq!(event.recipients.len(), 2);
        assert_eq!(event.idempotency_key.as_deref(), Some("order-42-confirm"));
    }

    #[test]
    fn ingest_event_minimal() {
        let json_str = r#"{
            "event": "user.signup",
            "recipients": [{"id": "u-1"}],
            "data": {}
        }"#;
        let event: IngestEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.event, "user.signup");
        assert!(event.idempotency_key.is_none());
        assert!(event.recipients[0].contacts.is_empty());
    }

    #[test]
    fn ingest_event_with_multiple_contacts() {
        let json_str = r#"{
            "event": "alert",
            "recipients": [{
                "id": "user-1",
                "contacts": {
                    "email": "a@b.com",
                    "sms": "+1234567890",
                    "telegram": "12345"
                }
            }],
            "data": {"message": "Server down"}
        }"#;
        let event: IngestEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.recipients[0].contacts.len(), 3);
    }
}
