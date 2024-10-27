mod admin;
mod ingest;
mod recipient;

use axum::{Extension, Router};
use notifico_core::http::SecretKey;
use notifico_core::pipeline::runner::ProcessEventRequest;
use notifico_subscription::SubscriptionManager;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub(crate) struct HttpExtensions {
    pub sender: Sender<ProcessEventRequest>,
    pub subman: Arc<SubscriptionManager>,
    pub secret_key: SecretKey,
}

pub(crate) async fn start(bind: SocketAddr, ext: HttpExtensions) {
    let app = Router::new().nest("/api/ingest", ingest::get_router(ext.clone()));
    let app = app.nest("/api/recipient", recipient::get_router(ext.clone()));
    let app = app.nest("/api/admin", admin::get_router(ext.clone()));
    let app = app.layer(Extension(Arc::new(ext.secret_key)));

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
