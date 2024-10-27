mod ingest;

use axum::Router;
use notifico_core::pipeline::runner::ProcessEventRequest;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;

pub(crate) async fn start(bind: SocketAddr, sender: Sender<ProcessEventRequest>) {
    let app = Router::new().nest("/api/ingest", ingest::get_router(sender));

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
