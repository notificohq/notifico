use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Extension, Json, Router};
use flume::Sender;
use notifico_core::engine::EventContext;
use notifico_core::pipeline::runner::ProcessEventRequest;
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_redoc::Redoc;
use utoipa_redoc::Servable;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct HttpExtensions {
    pub sender: Sender<ProcessEventRequest>,
}

#[derive(OpenApi)]
#[openapi(info(description = "Notifico Ingest API"), paths(send, send_webhook))]
struct ApiDoc;

pub(crate) async fn start(serviceapi_bind: SocketAddr, ext: HttpExtensions) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let listener = TcpListener::bind(serviceapi_bind).await.unwrap();

    // Service API
    let app = Router::new()
        .route("/v1/send", post(send))
        .route("/v1/send_webhook", post(send_webhook))
        .layer(Extension(ext.sender));

    let app =
        app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    let app = app.merge(Redoc::with_url("/redoc", ApiDoc::openapi()));

    tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
}

#[utoipa::path(post, path = "/v1/send")]
async fn send(
    Extension(sender): Extension<Sender<ProcessEventRequest>>,
    Json(payload): Json<ProcessEventRequest>,
) -> StatusCode {
    sender.send_async(payload).await.unwrap();

    StatusCode::ACCEPTED
}

#[derive(Deserialize)]
struct WebhookParameters {
    #[serde(default = "Uuid::nil")]
    project_id: Uuid,
    event: String,
}

#[utoipa::path(post, path = "/v1/send_webhook")]
async fn send_webhook(
    Extension(sender): Extension<Sender<ProcessEventRequest>>,
    parameters: Query<WebhookParameters>,
    Json(context): Json<EventContext>,
) -> StatusCode {
    let process_event_request = ProcessEventRequest {
        id: Uuid::now_v7(),
        project_id: parameters.project_id,
        event: parameters.event.clone(),
        recipient: None,
        context,
    };

    sender.send_async(process_event_request).await.unwrap();

    StatusCode::ACCEPTED
}
