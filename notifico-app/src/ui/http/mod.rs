mod admin;

use axum::http::header::CONTENT_TYPE;
use axum::http::{StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use axum::Router;
use notifico_dbpipeline::controllers::event::EventDbController;
use notifico_dbpipeline::controllers::pipeline::PipelineDbController;
use notifico_project::ProjectController;
use notifico_subscription::controllers::contact::ContactDbController;
use notifico_subscription::controllers::recipient::RecipientDbController;
use notifico_subscription::controllers::subscription::SubscriptionDbController;
use notifico_template::source::db::DbTemplateSource;
use rust_embed::Embed;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Clone)]
pub(crate) struct HttpUiExtensions {
    pub recipient_controller: Arc<RecipientDbController>,
    pub contact_controller: Arc<ContactDbController>,
    pub subscription_controller: Arc<SubscriptionDbController>,
    pub pipeline_controller: Arc<PipelineDbController>,
    pub project_controller: Arc<ProjectController>,
    pub template_controller: Arc<DbTemplateSource>,
    pub event_controller: Arc<EventDbController>,
}

#[derive(Embed)]
#[folder = "assets/"]
struct Assets;

pub(crate) async fn start(bind: SocketAddr, ext: HttpUiExtensions) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let service_listener = TcpListener::bind(bind).await.unwrap();

    // Service API
    let app = Router::new().nest("/api", admin::get_router(ext.clone()));
    let app = app.fallback(static_handler);

    tokio::spawn(async { axum::serve(service_listener, app).await.unwrap() });
}

const INDEX_HTML: &str = "index.html";

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }

    let Some(content) = Assets::get(path) else {
        return not_found().await;
    };

    ([(CONTENT_TYPE, content.metadata.mimetype())], content.data).into_response()
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
