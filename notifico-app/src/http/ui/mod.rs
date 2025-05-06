mod api;

use crate::controllers::api_key::ApiKeyController;
use crate::controllers::event::EventDbController;
use crate::controllers::pipeline::PipelineDbController;
use crate::controllers::project::ProjectController;
use axum::http::header::CONTENT_TYPE;
use axum::http::{StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::Router;
use axum_prometheus::PrometheusMetricLayer;
use rust_embed::Embed;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub(crate) struct HttpUiExtensions {
    pub pipeline_controller: Arc<PipelineDbController>,
    pub project_controller: Arc<ProjectController>,
    pub event_controller: Arc<EventDbController>,
    pub api_key_controller: Arc<ApiKeyController>,
}

#[derive(Embed)]
#[folder = "assets/"]
struct Assets;

pub(crate) async fn start(bind: SocketAddr, ext: HttpUiExtensions) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let service_listener = TcpListener::bind(bind).await.unwrap();

    // Service API
    let app = Router::new()
        .nest("/api", api::get_router(ext.clone()))
        .layer(PrometheusMetricLayer::new());
    let app = app.fallback(static_handler);

    tokio::spawn(async { axum::serve(service_listener, app).await.unwrap() });
}

const INDEX_HTML: &str = "index.html";

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if let Some(content) = Assets::get(path) {
        return ([(CONTENT_TYPE, content.metadata.mimetype())], content.data).into_response();
    };

    if path.is_empty() || path == INDEX_HTML || !path.starts_with("api/") {
        return index_html().await;
    }

    not_found().await
}

async fn index_html() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => Html(content.data).into_response(),
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "404").into_response()
}
