use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, put},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use notifico_db::repo::{admin, credential};

use crate::AppState;
use crate::auth::AuthContext;

type ApiResult = Result<Response, Response>;

pub fn admin_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/projects", get(list_projects).post(create_project))
        .route(
            "/projects/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
        .route("/events", get(list_events).post(create_event))
        .route("/events/{id}", put(update_event).delete(delete_event))
        .route(
            "/events/{event_id}/rules",
            get(list_rules).post(create_rule),
        )
        .route("/rules/{id}", put(update_rule).delete(delete_rule))
        .route("/templates", get(list_templates).post(create_template))
        .route("/templates/{id}", delete(delete_template))
        .route(
            "/templates/{template_id}/content/{locale}",
            put(set_template_content),
        )
        .route(
            "/credentials",
            get(list_credentials).post(create_credential),
        )
        .route("/credentials/{id}", delete(delete_credential))
}

fn require_admin(auth: &AuthContext) -> Result<(), Response> {
    auth.require_scope("admin")
        .map_err(|e| e.into_response())
}

fn db_err(e: sea_orm::DbErr) -> Response {
    tracing::error!(error = %e, "Database error");
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
}

fn not_found(msg: &str) -> Response {
    (StatusCode::NOT_FOUND, msg.to_string()).into_response()
}

// ── Projects ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ProjectResponse {
    id: Uuid,
    name: String,
    default_locale: String,
}

#[derive(Deserialize)]
struct CreateProjectRequest {
    name: String,
    #[serde(default = "default_locale")]
    default_locale: String,
}

fn default_locale() -> String {
    "en".into()
}

#[derive(Deserialize)]
struct UpdateProjectRequest {
    name: String,
    default_locale: String,
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
) -> ApiResult {
    require_admin(&auth)?;
    let projects = admin::list_projects(&state.db).await.map_err(db_err)?;
    Ok(Json(
        projects
            .into_iter()
            .map(|p| ProjectResponse {
                id: p.id,
                name: p.name,
                default_locale: p.default_locale,
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    let project = admin::get_project(&state.db, id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Project not found"))?;
    Ok(Json(ProjectResponse {
        id: project.id,
        name: project.name,
        default_locale: project.default_locale,
    })
    .into_response())
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(body): Json<CreateProjectRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    let id = Uuid::now_v7();
    admin::create_project(&state.db, id, &body.name, &body.default_locale)
        .await
        .map_err(db_err)?;
    Ok((
        StatusCode::CREATED,
        Json(ProjectResponse {
            id,
            name: body.name,
            default_locale: body.default_locale,
        }),
    )
        .into_response())
}

async fn update_project(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateProjectRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::update_project(&state.db, id, &body.name, &body.default_locale)
        .await
        .map_err(db_err)?;
    Ok(Json(ProjectResponse {
        id,
        name: body.name,
        default_locale: body.default_locale,
    })
    .into_response())
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::delete_project(&state.db, id)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ── Events ───────────────────────────────────────────────────────────

#[derive(Serialize)]
struct EventResponse {
    id: Uuid,
    project_id: Uuid,
    name: String,
    category: String,
    description: String,
}

#[derive(Deserialize)]
struct CreateEventRequest {
    name: String,
    category: String,
}

#[derive(Deserialize)]
struct UpdateEventRequest {
    name: String,
    category: String,
    #[serde(default)]
    description: String,
}

async fn list_events(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
) -> ApiResult {
    require_admin(&auth)?;
    let events = admin::list_events(&state.db, auth.project_id)
        .await
        .map_err(db_err)?;
    Ok(Json(
        events
            .into_iter()
            .map(|e| EventResponse {
                id: e.id,
                project_id: e.project_id,
                name: e.name,
                category: e.category,
                description: e.description,
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

async fn create_event(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(body): Json<CreateEventRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    let id = Uuid::now_v7();
    admin::create_event(&state.db, id, auth.project_id, &body.name, &body.category)
        .await
        .map_err(db_err)?;
    Ok((
        StatusCode::CREATED,
        Json(EventResponse {
            id,
            project_id: auth.project_id,
            name: body.name,
            category: body.category,
            description: String::new(),
        }),
    )
        .into_response())
}

async fn update_event(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateEventRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::update_event(&state.db, id, &body.name, &body.category, &body.description)
        .await
        .map_err(db_err)?;
    Ok(Json(EventResponse {
        id,
        project_id: auth.project_id,
        name: body.name,
        category: body.category,
        description: body.description,
    })
    .into_response())
}

async fn delete_event(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::delete_event(&state.db, id).await.map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ── Pipeline Rules ───────────────────────────────────────────────────

#[derive(Serialize)]
struct RuleResponse {
    id: Uuid,
    event_id: Uuid,
    channel: String,
    template_id: Uuid,
    enabled: bool,
    priority: i32,
}

#[derive(Deserialize)]
struct CreateRuleRequest {
    channel: String,
    template_id: Uuid,
    #[serde(default)]
    priority: i32,
}

#[derive(Deserialize)]
struct UpdateRuleRequest {
    channel: String,
    template_id: Uuid,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    priority: i32,
}

fn default_true() -> bool {
    true
}

async fn list_rules(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(event_id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    let rules = admin::list_rules(&state.db, event_id)
        .await
        .map_err(db_err)?;
    Ok(Json(
        rules
            .into_iter()
            .map(|r| RuleResponse {
                id: r.id,
                event_id: r.event_id,
                channel: r.channel,
                template_id: r.template_id,
                enabled: r.enabled,
                priority: r.priority,
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

async fn create_rule(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(event_id): Path<Uuid>,
    Json(body): Json<CreateRuleRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    let id = Uuid::now_v7();
    admin::create_rule(
        &state.db,
        id,
        event_id,
        &body.channel,
        body.template_id,
        body.priority,
    )
    .await
    .map_err(db_err)?;
    Ok((
        StatusCode::CREATED,
        Json(RuleResponse {
            id,
            event_id,
            channel: body.channel,
            template_id: body.template_id,
            enabled: true,
            priority: body.priority,
        }),
    )
        .into_response())
}

async fn update_rule(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRuleRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::update_rule(
        &state.db,
        id,
        &body.channel,
        body.template_id,
        body.enabled,
        body.priority,
    )
    .await
    .map_err(db_err)?;
    Ok(Json(serde_json::json!({"id": id, "updated": true})).into_response())
}

async fn delete_rule(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::delete_rule(&state.db, id).await.map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ── Templates ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct TemplateResponse {
    id: Uuid,
    project_id: Uuid,
    name: String,
    channel: String,
}

#[derive(Deserialize)]
struct CreateTemplateRequest {
    name: String,
    channel: String,
}

#[derive(Deserialize)]
struct SetContentRequest {
    body: Value,
}

async fn list_templates(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
) -> ApiResult {
    require_admin(&auth)?;
    let templates = admin::list_templates(&state.db, auth.project_id)
        .await
        .map_err(db_err)?;
    Ok(Json(
        templates
            .into_iter()
            .map(|t| TemplateResponse {
                id: t.id,
                project_id: t.project_id,
                name: t.name,
                channel: t.channel,
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

async fn create_template(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(body): Json<CreateTemplateRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    let id = Uuid::now_v7();
    admin::create_template(&state.db, id, auth.project_id, &body.name, &body.channel)
        .await
        .map_err(db_err)?;
    Ok((
        StatusCode::CREATED,
        Json(TemplateResponse {
            id,
            project_id: auth.project_id,
            name: body.name,
            channel: body.channel,
        }),
    )
        .into_response())
}

async fn delete_template(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::delete_template(&state.db, id)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn set_template_content(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path((template_id, locale)): Path<(Uuid, String)>,
    Json(body): Json<SetContentRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::set_template_content(&state.db, template_id, &locale, &body.body)
        .await
        .map_err(db_err)?;
    Ok(
        Json(serde_json::json!({"template_id": template_id, "locale": locale, "updated": true}))
            .into_response(),
    )
}

// ── Credentials ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct CredentialResponse {
    id: Uuid,
    name: String,
    channel: String,
    enabled: bool,
}

#[derive(Deserialize)]
struct CreateCredentialRequest {
    name: String,
    channel: String,
    data: Value,
}

async fn list_credentials(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
) -> ApiResult {
    require_admin(&auth)?;
    let creds = admin::list_credentials(&state.db, auth.project_id)
        .await
        .map_err(db_err)?;
    Ok(Json(
        creds
            .into_iter()
            .map(|c| CredentialResponse {
                id: c.id,
                name: c.name,
                channel: c.channel,
                enabled: c.enabled,
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

async fn create_credential(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(body): Json<CreateCredentialRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    let key = state.encryption_key.as_ref().ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Encryption key not configured. Set NOTIFICO_AUTH_ENCRYPTION_KEY.".to_string(),
        )
            .into_response()
    })?;
    let id = Uuid::now_v7();
    credential::insert_credential(
        &state.db,
        id,
        auth.project_id,
        &body.name,
        &body.channel,
        &body.data,
        key,
    )
    .await
    .map_err(db_err)?;
    Ok((
        StatusCode::CREATED,
        Json(CredentialResponse {
            id,
            name: body.name,
            channel: body.channel,
            enabled: true,
        }),
    )
        .into_response())
}

async fn delete_credential(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::delete_credential(&state.db, id)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
