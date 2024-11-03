use crate::http::HttpExtensions;
use axum::routing::{get, put};
use axum::{Extension, Router};
use tower_http::cors::CorsLayer;
pub mod subscription;

pub(crate) fn get_router(ext: HttpExtensions) -> Router {
    Router::new()
        .route("/v1/subscriptions", get(subscription::list_subscriptions))
        .route("/v1/subscriptions/:id", get(subscription::get_subscription))
        .route(
            "/v1/subscriptions/:id",
            put(subscription::update_subscription),
        )
        .layer(Extension(ext.subman))
        .layer(CorsLayer::permissive())
}
