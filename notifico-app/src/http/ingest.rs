use crate::controllers::api_key::{ApiKeyController, ApiKeyError};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{Extension, Json};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use axum_prometheus::PrometheusMetricLayer;
use notifico_core::pipeline::context::EventContext;
use notifico_core::pipeline::event::{ProcessEventRequest, RecipientSelector};
use notifico_core::queue::SenderChannel;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

#[derive(Clone)]
pub struct HttpIngestExtensions {
    pub(crate) sender: Arc<dyn SenderChannel>,
    pub(crate) api_key_controller: Arc<ApiKeyController>,
}

#[derive(OpenApi)]
#[openapi(info(
    title = "Notifico HTTP Ingest API",
    description = "This API is intended for triggering events and running pipelines for the corresponding events.",
    version = "0.2.0"
))]
struct ApiDoc;

pub async fn start(serviceapi_bind: SocketAddr, ext: HttpIngestExtensions) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let listener = TcpListener::bind(serviceapi_bind).await.unwrap();

    // Service API
    let app = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(trigger))
        .routes(routes!(trigger_webhook))
        .layer(PrometheusMetricLayer::new())
        .layer(Extension(ext));

    let (mut app, api) = app.split_for_parts();
    app = app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api));

    tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
struct TriggerEvent {
    #[serde(default = "Uuid::now_v7")]
    pub id: Uuid,
    pub event: String,
    #[serde(default = "Vec::new")]
    pub recipients: Vec<RecipientSelector>,
    #[serde(default)]
    pub context: EventContext,
}

#[utoipa::path(
    post,
    path = "/v1/trigger",
    tag = "trigger",
    responses(
        (status = StatusCode::ACCEPTED, description = "Event sent successfully"),
    ),
    description = "Send an event for processing. An available worker will then run the pipelines for the corresponding event.

In standalone configuration, the event is queued using an in-memory queue.
In AMQP configuration, the event is sent to the AMQP queue.",
)]
async fn trigger(
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
    Extension(ext): Extension<HttpIngestExtensions>,
    Json(payload): Json<TriggerEvent>,
) -> StatusCode {
    let project_id = ext
        .api_key_controller
        .authorize_api_key(auth_header.token())
        .await;

    let project_id = match project_id {
        Ok(project_id) => project_id,
        Err(ApiKeyError::InvalidApiKey) => return StatusCode::FORBIDDEN,
        Err(ApiKeyError::InternalError) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let process_event_request = ProcessEventRequest {
        id: payload.id,
        project_id,
        event: payload.event,
        recipients: payload.recipients,
        context: payload.context,
    };

    ext.sender.send(process_event_request).await.unwrap();

    StatusCode::ACCEPTED
}

#[derive(Deserialize, IntoParams)]
#[into_params(style = Form, parameter_in = Query)]
struct WebhookParameters {
    event: String,
    token: String,
}

#[utoipa::path(
    post,
    path = "/v1/trigger/webhook",
    tag = "trigger",
    params(WebhookParameters),
    responses(
        (status = StatusCode::ACCEPTED, description = "Event sent successfully"),
    ),
    description = "This method accepts any JSON as POST body as a context, so you can use it in your template.

Recipients must be set in Pipeline, using `core.set_recipients` step. This method is intended for any external system that accepts webhook integration,
so you can create notifications for arbitrary webhooks."
)]
async fn trigger_webhook(
    Extension(ext): Extension<HttpIngestExtensions>,
    parameters: Query<WebhookParameters>,
    Json(context): Json<EventContext>,
) -> StatusCode {
    let project_id = ext
        .api_key_controller
        .authorize_api_key(&parameters.token)
        .await;

    let project_id = match project_id {
        Ok(project_id) => project_id,
        Err(ApiKeyError::InvalidApiKey) => return StatusCode::FORBIDDEN,
        Err(ApiKeyError::InternalError) => return StatusCode::INTERNAL_SERVER_ERROR,
    };

    let process_event_request = ProcessEventRequest {
        id: Uuid::now_v7(),
        project_id,
        event: parameters.event.clone(),
        recipients: vec![],
        context,
    };

    ext.sender.send(process_event_request).await.unwrap();

    StatusCode::ACCEPTED
}
