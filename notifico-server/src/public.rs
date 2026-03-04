use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use notifico_db::repo::{preference, recipient};

use crate::AppState;
use crate::auth::AuthContext;

type ApiResult = Result<Response, Response>;

pub fn public_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/preferences", get(get_preferences).put(set_preference))
        .route("/unsubscribe", get(unsubscribe_get).post(unsubscribe_post))
}

fn db_err(e: sea_orm::DbErr) -> Response {
    tracing::error!(error = %e, "Database error");
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}

fn bad_request(msg: &str) -> Response {
    (StatusCode::BAD_REQUEST, msg.to_string()).into_response()
}

// ── Preferences ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PreferenceQuery {
    recipient: String,
}

#[derive(Serialize)]
struct PreferenceResponse {
    category: String,
    channel: String,
    enabled: bool,
}

async fn get_preferences(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Query(q): Query<PreferenceQuery>,
) -> ApiResult {
    // Find recipient by external_id
    let recipient_row =
        recipient::find_by_external_id(&state.db, auth.project_id, &q.recipient)
            .await
            .map_err(db_err)?
            .ok_or_else(|| bad_request("Recipient not found"))?;

    let prefs = preference::list_preferences(&state.db, recipient_row.id)
        .await
        .map_err(db_err)?;

    Ok(Json(
        prefs
            .into_iter()
            .map(|p| PreferenceResponse {
                category: p.category,
                channel: p.channel,
                enabled: p.enabled,
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

#[derive(Deserialize)]
struct SetPreferenceRequest {
    recipient: String,
    category: String,
    channel: String,
    enabled: bool,
}

async fn set_preference(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(body): Json<SetPreferenceRequest>,
) -> ApiResult {
    let recipient_row =
        recipient::find_by_external_id(&state.db, auth.project_id, &body.recipient)
            .await
            .map_err(db_err)?
            .ok_or_else(|| bad_request("Recipient not found"))?;

    preference::set_preference(
        &state.db,
        recipient_row.id,
        &body.category,
        &body.channel,
        body.enabled,
    )
    .await
    .map_err(db_err)?;

    Ok(Json(serde_json::json!({"updated": true})).into_response())
}

// ── Unsubscribe ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct UnsubscribeQuery {
    token: String,
}

#[derive(Deserialize)]
struct UnsubscribeRequest {
    token: String,
}

/// GET /unsubscribe?token=... — one-click unsubscribe for email links (RFC 8058).
async fn unsubscribe_get(
    State(state): State<Arc<AppState>>,
    Query(q): Query<UnsubscribeQuery>,
) -> ApiResult {
    let applied = preference::apply_unsubscribe(&state.db, &q.token)
        .await
        .map_err(db_err)?;

    if applied {
        Ok((StatusCode::OK, "You have been unsubscribed.").into_response())
    } else {
        Ok((StatusCode::NOT_FOUND, "Invalid or expired unsubscribe link.").into_response())
    }
}

/// POST /unsubscribe — programmatic unsubscribe.
async fn unsubscribe_post(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UnsubscribeRequest>,
) -> ApiResult {
    let applied = preference::apply_unsubscribe(&state.db, &body.token)
        .await
        .map_err(db_err)?;

    Ok(Json(serde_json::json!({"unsubscribed": applied})).into_response())
}
