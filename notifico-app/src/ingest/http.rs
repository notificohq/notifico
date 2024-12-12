use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Extension, Json, Router};
use notifico_core::engine::EventContext;
use notifico_core::pipeline::event::ProcessEventRequest;
use notifico_core::queue::SenderChannel;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use utoipa::{IntoParams, OpenApi};
use utoipa_redoc::Redoc;
use utoipa_redoc::Servable;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

#[derive(Clone)]
pub struct HttpIngestExtensions {
    pub(crate) sender: Arc<dyn SenderChannel>,
}

#[derive(OpenApi)]
#[openapi(info(description = "Notifico Ingest API"), paths(send, send_webhook))]
struct ApiDoc;

pub async fn start(serviceapi_bind: SocketAddr, ext: HttpIngestExtensions) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let listener = TcpListener::bind(serviceapi_bind).await.unwrap();

    // Service API
    let app = Router::new()
        .route("/v1/send", post(send))
        .route("/v1/send_webhook", post(send_webhook))
        .layer(Extension(ext));

    let app =
        app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    let app = app.merge(Redoc::with_url("/redoc", ApiDoc::openapi()));

    tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
}

#[utoipa::path(
    post,
    path = "/v1/send",
    description = "Send an event to the AMQP event bus. Available worker will then run the pipelines for the corresponding event"
)]
async fn send(
    Extension(ext): Extension<HttpIngestExtensions>,
    Json(payload): Json<ProcessEventRequest>,
) -> StatusCode {
    ext.sender
        .send(serde_json::to_string(&payload).unwrap())
        .await
        .unwrap();

    StatusCode::ACCEPTED
}

#[derive(Deserialize, IntoParams)]
#[into_params(style = Form, parameter_in = Query)]
struct WebhookParameters {
    #[serde(default = "Uuid::nil")]
    project_id: Uuid,
    event: String,
}

#[utoipa::path(
    post,
    path = "/v1/send_webhook",
    params(WebhookParameters),
    description = "This method accepts any JSON as POST body as a context, so you can use it in your template.

Recipients must be set in Pipeline, using `core.set_recipients` step. This method is intended for any external system that accepts webhook integration,
so you can create notifications for arbitrary webhooks."
)]
async fn send_webhook(
    Extension(ext): Extension<HttpIngestExtensions>,
    parameters: Query<WebhookParameters>,
    Json(context): Json<EventContext>,
) -> StatusCode {
    let process_event_request = ProcessEventRequest {
        id: Uuid::now_v7(),
        project_id: parameters.project_id,
        event: parameters.event.clone(),
        recipients: vec![],
        context,
    };

    ext.sender
        .send(serde_json::to_string(&process_event_request).unwrap())
        .await
        .unwrap();

    StatusCode::ACCEPTED
}
