use axum::{http::StatusCode, routing::post, Extension, Json, Router};
use notifico_core::pipeline::runner::ProcessEventRequest;
use std::net::SocketAddr;
use tokio::sync::mpsc::Sender;

pub(crate) async fn start(bind: SocketAddr, sender: Sender<ProcessEventRequest>) {
    let ingest = Router::new()
        .route("/v1/send", post(send))
        .layer(Extension(sender));

    let app = Router::new().nest("/api/ingest", ingest);

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn send(
    Extension(sender): Extension<Sender<ProcessEventRequest>>,
    Json(payload): Json<ProcessEventRequest>,
) -> StatusCode {
    sender.send(payload).await.unwrap();
    // state.runner.process_eventrequest(payload).await;

    StatusCode::CREATED
}
