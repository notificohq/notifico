mod admin;
mod ingest;
mod recipient;

use axum::http::header::CONTENT_TYPE;
use axum::http::{StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::{Extension, Router};
use notifico_core::http::SecretKey;
use notifico_core::pipeline::runner::ProcessEventRequest;
use notifico_core::pipeline::storage::PipelineStorage;
use notifico_project::ProjectController;
use notifico_subscription::SubscriptionManager;
use notifico_template::source::TemplateSource;
use rust_embed::Embed;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::mpsc::Sender;
use utoipa::OpenApi;
use utoipa_redoc::Redoc;
use utoipa_redoc::Servable;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub(crate) struct HttpExtensions {
    pub sender: Sender<ProcessEventRequest>,
    pub subman: Arc<SubscriptionManager>,
    pub secret_key: Arc<SecretKey>,
    pub pipeline_storage: Arc<dyn PipelineStorage>,
    pub projects_controller: Arc<ProjectController>,
    pub templates_controller: Arc<dyn TemplateSource>,
}

#[derive(OpenApi)]
#[openapi(info(description = "Notifico Service API"), paths(ingest::send))]
struct ApiDoc;

#[derive(Embed)]
#[folder = "admin/"]
struct AdminAssets;

pub(crate) async fn start(
    serviceapi_bind: SocketAddr,
    clientapi_bind: SocketAddr,
    ext: HttpExtensions,
) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let service_listener = TcpListener::bind(serviceapi_bind).await.unwrap();
    let client_listener = TcpListener::bind(clientapi_bind).await.unwrap();

    // Service API
    let app = Router::new().nest("/api", ingest::get_router(ext.clone()));
    let app = app.nest("/api", admin::get_router(ext.clone()));
    let app = app.layer(Extension(ext.secret_key.clone()));

    let app =
        app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    let app = app.merge(Redoc::with_url("/redoc", ApiDoc::openapi()));
    let app = app.fallback(static_handler);

    tokio::spawn(async { axum::serve(service_listener, app).await.unwrap() });

    // Client API
    let app = Router::new().nest("/api", recipient::get_router(ext.clone()));
    let app = app.layer(Extension(ext.secret_key.clone()));

    tokio::spawn(async { axum::serve(client_listener, app).await.unwrap() });
}

const INDEX_HTML: &str = "index.html";

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }

    let Some(content) = AdminAssets::get(path) else {
        return not_found().await;
    };

    ([(CONTENT_TYPE, content.metadata.mimetype())], content.data).into_response()
}

async fn index_html() -> Response {
    match AdminAssets::get(INDEX_HTML) {
        Some(content) => Html(content.data).into_response(),
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "404").into_response()
}
