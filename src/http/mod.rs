use crate::event_handler::{EventHandler, ProcessEventRequest};
use actix::Addr;
use axum::extract::State;
use axum::{http::StatusCode, routing::post, Json, Router};
use std::net::SocketAddr;

#[derive(Clone)]
struct SharedState {
    event_handler: Addr<EventHandler>,
}

pub(crate) async fn start(event_handler: Addr<EventHandler>, bind: SocketAddr) {
    let state = SharedState { event_handler };

    let ingest = Router::new().route("/v1/send", post(send));

    let app = Router::new().nest("/ingest", ingest).with_state(state);

    let listener = tokio::net::TcpListener::bind(bind).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn send(
    State(state): State<SharedState>,
    Json(payload): Json<ProcessEventRequest>,
) -> StatusCode {
    state.event_handler.send(payload).await.unwrap();

    StatusCode::CREATED
}
