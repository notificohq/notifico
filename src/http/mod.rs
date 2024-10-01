use crate::event_handler::{EventHandler, ProcessEvent};
use crate::http::list_unsubscribe::list_unsubscribe;
use actix::Addr;
use axum::extract::State;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use hmac::Hmac;
use notifico_subscription::SubscriptionManager;
use serde::Serialize;
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

mod list_unsubscribe;

#[derive(Clone)]
struct SharedState {
    event_handler: Addr<EventHandler>,
    sub_manager: Arc<SubscriptionManager>,
    secret_key: Hmac<Sha256>,
}

pub(crate) async fn start(
    event_handler: Addr<EventHandler>,
    sub_manager: Arc<SubscriptionManager>,
    secret_key: Hmac<Sha256>,
) {
    let state = SharedState {
        event_handler,
        sub_manager,
        secret_key,
    };

    // build our application with a route
    let app = Router::new()
        .route("/", get(root))
        .route("/trigger", post(trigger))
        .route("/unsubscribe", get(list_unsubscribe))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}

async fn trigger(
    State(state): State<SharedState>,
    Json(payload): Json<ProcessEvent>,
) -> (StatusCode, Json<User>) {
    let user = User {
        id: payload.project_id,
    };

    state.event_handler.send(payload).await.unwrap();

    (StatusCode::CREATED, Json(user))
}

#[derive(Serialize)]
struct User {
    id: Uuid,
}
