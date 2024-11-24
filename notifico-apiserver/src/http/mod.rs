mod ingest;
mod list_unsubscribe;
mod recipient;

use axum::{Extension, Router};
use notifico_core::http::SecretKey;
use notifico_core::pipeline::runner::ProcessEventRequest;
use notifico_subscription::SubscriptionManager;
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
}

#[derive(OpenApi)]
#[openapi(info(description = "Notifico Service API"), paths(ingest::send))]
struct ApiDoc;

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
    let app = app.layer(Extension(ext.secret_key.clone()));

    let app =
        app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
    let app = app.merge(Redoc::with_url("/redoc", ApiDoc::openapi()));

    tokio::spawn(async { axum::serve(service_listener, app).await.unwrap() });

    // Client API
    let app = Router::new().nest("/api", recipient::get_router(ext.clone()));
    let app = app.layer(Extension(ext.secret_key.clone()));

    tokio::spawn(async { axum::serve(client_listener, app).await.unwrap() });
}
