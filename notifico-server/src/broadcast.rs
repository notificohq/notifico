use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use notifico_core::pipeline::{PipelineInput, execute_pipeline};
use notifico_db::repo;

use crate::AppState;
use crate::auth::AuthContext;

#[derive(Debug, Deserialize, ToSchema)]
pub struct BroadcastRequest {
    /// Event name to trigger
    pub event: String,
    /// Template data
    pub data: serde_json::Value,
    /// Optional list of recipient external IDs to target.
    /// If omitted, sends to all recipients in the project.
    #[serde(default)]
    pub recipients: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BroadcastResponse {
    pub broadcast_id: Uuid,
    pub recipient_count: usize,
    pub task_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/broadcasts",
    tag = "broadcasts",
    request_body = BroadcastRequest,
    responses(
        (status = 200, description = "Broadcast enqueued", body = BroadcastResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Event not found"),
        (status = 429, description = "Rate limited"),
    ),
    security(("bearer" = []))
)]
pub async fn handle_broadcast(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(req): Json<BroadcastRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    auth.require_scope("ingest")
        .map_err(|e| (StatusCode::FORBIDDEN, format!("{e:?}")))?;

    if let Err(retry_after) = state.rate_limiter.check(auth.api_key_id) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!("Rate limit exceeded. Retry after {retry_after}s"),
        ));
    }

    let project_id = auth.project_id;
    let default_locale = &state.config.project.default_locale;
    let broadcast_id = Uuid::now_v7();

    // Resolve event
    let event_row = repo::template::find_event_by_name(&state.db, project_id, &req.event)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Event not found: {}", req.event)))?;

    // Get pipeline rules
    let rules = repo::template::get_pipeline_rules(&state.db, event_row.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if rules.is_empty() {
        return Ok(Json(BroadcastResponse {
            broadcast_id,
            recipient_count: 0,
            task_count: 0,
            errors: vec![format!("No pipeline rules for event: {}", req.event)],
        }));
    }

    // Resolve recipients
    let all_recipients = repo::admin::list_recipients(&state.db, project_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let recipients: Vec<_> = if let Some(ref filter_ids) = req.recipients {
        all_recipients
            .into_iter()
            .filter(|r| filter_ids.contains(&r.external_id))
            .collect()
    } else {
        all_recipients
    };

    let recipient_count = recipients.len();
    let mut task_ids = Vec::new();
    let mut errors = Vec::new();

    for recipient in &recipients {
        let recipient_id = recipient.id;
        let recipient_locale = if recipient.locale.is_empty() {
            default_locale.as_str()
        } else {
            &recipient.locale
        };

        // Get contacts from DB
        let db_contacts = match repo::recipient::get_contacts(&state.db, recipient_id).await {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!(
                    "Failed to get contacts for {}: {}",
                    recipient.external_id, e
                ));
                continue;
            }
        };

        for rule in &rules {
            // Find contact for this channel
            let contact_value = match db_contacts.iter().find(|c| c.channel == rule.channel) {
                Some(c) => c.value.clone(),
                None => continue, // Skip silently — no contact for this channel
            };

            // Check preferences
            if event_row.category != "transactional" {
                match repo::preference::is_opted_out(
                    &state.db,
                    recipient_id,
                    &event_row.category,
                    &rule.channel,
                )
                .await
                {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, "Preference check failed, proceeding");
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
                        "Template not found for rule {} locale {}",
                        rule.id, recipient_locale
                    ));
                    continue;
                }
                Err(e) => {
                    errors.push(format!("Template error: {}", e));
                    continue;
                }
            };

            // Execute pipeline
            let pipeline_input = PipelineInput {
                project_id,
                event_name: req.event.clone(),
                recipient_id,
                recipient_locale: recipient_locale.to_string(),
                channel: rule.channel.clone(),
                contact_value,
                template_body: template.body,
                context_data: req.data.clone(),
                idempotency_key: None,
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
                        errors.push(format!("Enqueue error: {}", e));
                        continue;
                    }
                    task_ids.push(output.id);
                }
                Err(e) => {
                    errors.push(format!(
                        "Pipeline error for {}: {}",
                        recipient.external_id, e
                    ));
                }
            }
        }
    }

    tracing::info!(
        broadcast_id = %broadcast_id,
        recipient_count = recipient_count,
        task_count = task_ids.len(),
        "Broadcast enqueued"
    );

    Ok(Json(BroadcastResponse {
        broadcast_id,
        recipient_count,
        task_count: task_ids.len(),
        errors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcast_request_parsing() {
        let json = r#"{
            "event": "newsletter.weekly",
            "data": {"subject": "Weekly Update"},
            "recipients": ["user-1", "user-2"]
        }"#;
        let req: BroadcastRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.event, "newsletter.weekly");
        assert_eq!(req.recipients.unwrap().len(), 2);
    }

    #[test]
    fn broadcast_request_without_filter() {
        let json = r#"{
            "event": "promo.sale",
            "data": {"discount": "20%"}
        }"#;
        let req: BroadcastRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.event, "promo.sale");
        assert!(req.recipients.is_none());
    }
}
