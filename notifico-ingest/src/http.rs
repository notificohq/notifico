use axum::http::StatusCode;
use axum::routing::post;
use axum::{Extension, Json, Router};
use flume::Sender;
use notifico_core::pipeline::runner::ProcessEventRequest;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa_redoc::Redoc;
use utoipa_redoc::Servable;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub(crate) struct HttpExtensions {
    pub sender: Sender<ProcessEventRequest>,
}

#[derive(OpenApi)]
#[openapi(info(description = "Notifico Ingest API"), paths(event_send))]
struct ApiDoc;

pub(crate) async fn start(serviceapi_bind: SocketAddr, ext: HttpExtensions) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let listener = TcpListener::bind(serviceapi_bind).await.unwrap();

    // Service API
    let app = Router::new()
        .route("/v1/send", post(event_send))
        .layer(Extension(ext.sender));

    let app =
        app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    let app = app.merge(Redoc::with_url("/redoc", ApiDoc::openapi()));

    tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
}

#[utoipa::path(post, path = "/v1/send")]
async fn event_send(
    Extension(sender): Extension<Sender<ProcessEventRequest>>,
    Json(payload): Json<ProcessEventRequest>,
) -> StatusCode {
    sender.send_async(payload).await.unwrap();

    StatusCode::ACCEPTED
}
