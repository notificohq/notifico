use crate::event_handler::{EventHandler, ProcessEventRequest};
use actix::Addr;
use axum::extract::State;
use axum::{http::StatusCode, middleware, routing::post, Extension, Json, Router};
use notifico_core::http::{auth, SecretKey};
use notifico_ncenter::http::get_extapi_router as ncenter_router;
use notifico_ncenter::NCenterPlugin;
use notifico_subscription::http::get_router as subscription_router;
use notifico_subscription::SubscriptionManager;
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
struct SharedState {
    event_handler: Addr<EventHandler>,
}

pub(crate) async fn start(
    event_handler: Addr<EventHandler>,
    sub_manager: Arc<SubscriptionManager>,
    secret_key: Vec<u8>,
    ncenter: Arc<NCenterPlugin>,
    bind: SocketAddr,
) {
    let state = SharedState { event_handler };

    let extapi = Router::new()
        .nest("/subscription", subscription_router(sub_manager))
        .nest("/ncenter", ncenter_router(ncenter))
        .layer(middleware::from_fn(auth::authorize))
        .layer(Extension(Arc::new(SecretKey(secret_key))));

    let ingest = Router::new().route("/send", post(send));

    let app = Router::new()
        .nest("/ingest", ingest)
        .nest("/extapi", extapi)
        .with_state(state);

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
