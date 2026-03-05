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

use axum::extract::Query;

use notifico_db::repo::{admin, api_key, credential, delivery_log};

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
        // Recipients
        .route(
            "/recipients",
            get(list_recipients).post(create_recipient),
        )
        .route(
            "/recipients/{id}",
            get(get_recipient).put(update_recipient).delete(delete_recipient),
        )
        .route(
            "/recipients/{recipient_id}/contacts",
            get(list_contacts).post(add_contact),
        )
        .route("/contacts/{id}", delete(delete_contact))
        // Delivery log
        .route("/delivery-log", get(query_delivery_log))
        // API keys
        .route("/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api-keys/{id}", delete(delete_api_key))
        // Template preview
        .route("/templates/{id}/preview", axum::routing::post(preview_template))
        // Event stats
        .route("/events/{id}/stats", get(event_stats))
        // Channels
        .route("/channels", get(list_channels))
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

// ── Recipients ──────────────────────────────────────────────────────

#[derive(Serialize)]
struct RecipientResponse {
    id: Uuid,
    external_id: String,
    locale: String,
    timezone: String,
    metadata: Value,
}

#[derive(Deserialize)]
struct CreateRecipientRequest {
    external_id: String,
    #[serde(default = "default_locale")]
    locale: String,
    #[serde(default = "default_timezone")]
    timezone: String,
}

fn default_timezone() -> String {
    "UTC".into()
}

#[derive(Deserialize)]
struct UpdateRecipientRequest {
    locale: String,
    timezone: String,
    #[serde(default)]
    metadata: Value,
}

async fn list_recipients(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
) -> ApiResult {
    require_admin(&auth)?;
    let recipients = admin::list_recipients(&state.db, auth.project_id)
        .await
        .map_err(db_err)?;
    Ok(Json(
        recipients
            .into_iter()
            .map(|r| RecipientResponse {
                id: r.id,
                external_id: r.external_id,
                locale: r.locale,
                timezone: r.timezone,
                metadata: r.metadata,
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

async fn get_recipient(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    let recipient = admin::get_recipient(&state.db, id)
        .await
        .map_err(db_err)?
        .ok_or_else(|| not_found("Recipient not found"))?;
    Ok(Json(RecipientResponse {
        id: recipient.id,
        external_id: recipient.external_id,
        locale: recipient.locale,
        timezone: recipient.timezone,
        metadata: recipient.metadata,
    })
    .into_response())
}

async fn create_recipient(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(body): Json<CreateRecipientRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    let id = Uuid::now_v7();
    admin::create_recipient(
        &state.db,
        id,
        auth.project_id,
        &body.external_id,
        &body.locale,
        &body.timezone,
    )
    .await
    .map_err(db_err)?;
    Ok((
        StatusCode::CREATED,
        Json(RecipientResponse {
            id,
            external_id: body.external_id,
            locale: body.locale,
            timezone: body.timezone,
            metadata: Value::Object(Default::default()),
        }),
    )
        .into_response())
}

async fn update_recipient(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateRecipientRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::update_recipient(&state.db, id, &body.locale, &body.timezone, &body.metadata)
        .await
        .map_err(db_err)?;
    Ok(Json(serde_json::json!({"id": id, "updated": true})).into_response())
}

async fn delete_recipient(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::delete_recipient(&state.db, id)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ── Contacts ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ContactResponse {
    id: Uuid,
    channel: String,
    value: String,
    verified: bool,
}

#[derive(Deserialize)]
struct AddContactRequest {
    channel: String,
    value: String,
}

async fn list_contacts(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(recipient_id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    let contacts = admin::list_contacts(&state.db, recipient_id)
        .await
        .map_err(db_err)?;
    Ok(Json(
        contacts
            .into_iter()
            .map(|c| ContactResponse {
                id: c.id,
                channel: c.channel,
                value: c.value,
                verified: c.verified,
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

async fn add_contact(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(recipient_id): Path<Uuid>,
    Json(body): Json<AddContactRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    let id = Uuid::now_v7();
    admin::add_contact(&state.db, id, recipient_id, &body.channel, &body.value)
        .await
        .map_err(db_err)?;
    Ok((
        StatusCode::CREATED,
        Json(ContactResponse {
            id,
            channel: body.channel,
            value: body.value,
            verified: false,
        }),
    )
        .into_response())
}

async fn delete_contact(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::delete_contact(&state.db, id)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ── Delivery Log ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct DeliveryLogQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    event: Option<String>,
    #[serde(default = "default_limit")]
    limit: u64,
    #[serde(default)]
    offset: u64,
}

fn default_limit() -> u64 {
    50
}

#[derive(Serialize)]
struct DeliveryLogResponse {
    id: Uuid,
    event_name: String,
    recipient_id: Uuid,
    channel: String,
    status: String,
    error_message: Option<String>,
    attempts: i32,
    created_at: String,
    delivered_at: Option<String>,
}

#[derive(Serialize)]
struct DeliveryLogPage {
    items: Vec<DeliveryLogResponse>,
    total: i64,
    limit: u64,
    offset: u64,
}

async fn query_delivery_log(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Query(q): Query<DeliveryLogQuery>,
) -> ApiResult {
    require_admin(&auth)?;
    let logs = delivery_log::list_logs(
        &state.db,
        auth.project_id,
        q.status.as_deref(),
        q.event.as_deref(),
        q.limit.min(200),
        q.offset,
    )
    .await
    .map_err(db_err)?;

    let total = delivery_log::count_logs(
        &state.db,
        auth.project_id,
        q.status.as_deref(),
        q.event.as_deref(),
    )
    .await
    .map_err(db_err)?;

    Ok(Json(DeliveryLogPage {
        items: logs
            .into_iter()
            .map(|l| DeliveryLogResponse {
                id: l.id,
                event_name: l.event_name,
                recipient_id: l.recipient_id,
                channel: l.channel,
                status: l.status,
                error_message: l.error_message,
                attempts: l.attempts,
                created_at: l.created_at,
                delivered_at: l.delivered_at,
            })
            .collect(),
        total,
        limit: q.limit,
        offset: q.offset,
    })
    .into_response())
}

// ── API Keys ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ApiKeyResponse {
    id: Uuid,
    name: String,
    key_prefix: String,
    scope: String,
    enabled: bool,
}

#[derive(Serialize)]
struct ApiKeyCreatedResponse {
    id: Uuid,
    name: String,
    scope: String,
    raw_key: String,
}

#[derive(Deserialize)]
struct CreateApiKeyRequest {
    name: String,
    #[serde(default = "default_scope")]
    scope: String,
}

fn default_scope() -> String {
    "ingest".into()
}

async fn list_api_keys(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
) -> ApiResult {
    require_admin(&auth)?;
    let keys = admin::list_api_keys(&state.db, auth.project_id)
        .await
        .map_err(db_err)?;
    Ok(Json(
        keys.into_iter()
            .map(|k| ApiKeyResponse {
                id: k.id,
                name: k.name,
                key_prefix: k.key_prefix,
                scope: k.scope,
                enabled: k.enabled,
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

async fn create_api_key(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(body): Json<CreateApiKeyRequest>,
) -> ApiResult {
    require_admin(&auth)?;
    let id = Uuid::now_v7();
    let raw_key = format!("nk_live_{}", Uuid::now_v7().simple());
    api_key::insert_api_key(
        &state.db,
        id,
        auth.project_id,
        &body.name,
        &raw_key,
        &body.scope,
    )
    .await
    .map_err(db_err)?;
    Ok((
        StatusCode::CREATED,
        Json(ApiKeyCreatedResponse {
            id,
            name: body.name,
            scope: body.scope,
            raw_key,
        }),
    )
        .into_response())
}

async fn delete_api_key(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;
    admin::delete_api_key(&state.db, id)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// ── Channels ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChannelResponse {
    channel_id: String,
    display_name: String,
    content_schema: Value,
    credential_schema: Value,
}

async fn list_channels(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
) -> ApiResult {
    require_admin(&auth)?;
    let info = state.registry.channel_info();
    Ok(Json(
        info.into_iter()
            .map(|ch| ChannelResponse {
                channel_id: ch.channel_id.to_string(),
                display_name: ch.display_name,
                content_schema: serde_json::to_value(&ch.content_schema).unwrap_or_default(),
                credential_schema: serde_json::to_value(&ch.credential_schema)
                    .unwrap_or_default(),
            })
            .collect::<Vec<_>>(),
    )
    .into_response())
}

// --- Template Preview ---

#[derive(Deserialize)]
struct PreviewRequest {
    locale: Option<String>,
    data: Value,
}

#[derive(Serialize)]
struct PreviewResponse {
    rendered: Value,
}

async fn preview_template(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(template_id): Path<Uuid>,
    Json(req): Json<PreviewRequest>,
) -> ApiResult {
    require_admin(&auth)?;

    let locale = req.locale.as_deref().unwrap_or("en");
    let default_locale = &state.config.project.default_locale;

    let template = notifico_db::repo::template::resolve_template(
        &state.db,
        template_id,
        locale,
        default_locale,
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?
    .ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            format!("Template not found: {template_id} locale {locale}"),
        )
            .into_response()
    })?;

    let rendered = notifico_template::render_body(&template.body, &req.data)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Render error: {e}")).into_response())?;

    Ok(Json(PreviewResponse {
        rendered: Value::Object(rendered),
    })
    .into_response())
}

// --- Event Stats ---

#[derive(Serialize)]
struct EventStatsResponse {
    event_id: Uuid,
    stats: Vec<StatusCount>,
}

#[derive(Serialize)]
struct StatusCount {
    status: String,
    count: i64,
}

async fn event_stats(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Path(event_id): Path<Uuid>,
) -> ApiResult {
    require_admin(&auth)?;

    // Look up the event to get its name
    let event = admin::get_event(&state.db, event_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Event not found").into_response())?;

    let counts = delivery_log::count_by_event_name(&state.db, auth.project_id, &event.name)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Json(EventStatsResponse {
        event_id,
        stats: counts
            .into_iter()
            .map(|(status, count)| StatusCount { status, count })
            .collect(),
    })
    .into_response())
}
