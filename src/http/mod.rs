use axum::extract::State;
use axum::{http::StatusCode, routing::post, Json, Router};
use notifico_core::pipeline::runner::{PipelineRunner, ProcessEventRequest};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
struct SharedState {
    runner: Arc<PipelineRunner>,
}

pub(crate) async fn start(runner: Arc<PipelineRunner>, bind: SocketAddr) {
    let state = SharedState { runner };

    let ingest = Router::new().route("/v1/send", post(send));

    let app = Router::new().nest("/api/ingest", ingest).with_state(state);

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn send(
    State(state): State<SharedState>,
    Json(payload): Json<ProcessEventRequest>,
) -> StatusCode {
    state.runner.process_eventrequest(payload).await;

    StatusCode::CREATED
}
