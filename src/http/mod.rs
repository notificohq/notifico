use crate::event_handler::{EventHandler, ProcessEvent};
use actix::Addr;
use axum::extract::State;
use axum::handler::Handler;
use axum::{http::StatusCode, middleware, routing::post, Extension, Json, Router};
use hmac::Hmac;
use notifico_ncenter::http::get_router as ncenter_router;
use notifico_ncenter::NCenterPlugin;
use notifico_subscription::http::get_router as subscription_router;
use notifico_subscription::SubscriptionManager;
use serde::Serialize;
use sha2::Sha256;
use std::sync::Arc;
use uuid::Uuid;

mod auth;

#[derive(Clone)]
struct SharedState {
    event_handler: Addr<EventHandler>,
}

#[derive(Clone)]
struct SecretKey {
    secret_key: Hmac<Sha256>,
}

pub(crate) async fn start(
    event_handler: Addr<EventHandler>,
    sub_manager: Arc<SubscriptionManager>,
    secret_key: Hmac<Sha256>,
    ncenter: Arc<NCenterPlugin>,
) {
    let state = SharedState { event_handler };

    let extapi = Router::new()
        .nest("/subscription", subscription_router(sub_manager))
        .nest("/ncenter", ncenter_router(ncenter))
        .layer(middleware::from_fn(auth::authorize))
        .layer(Extension(Arc::new(SecretKey { secret_key })));

    // build our application with a route
    let app = Router::new()
        .route("/trigger", post(trigger))
        .nest("/extapi", extapi)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
