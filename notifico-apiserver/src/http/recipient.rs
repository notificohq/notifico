use crate::http::HttpExtensions;
use axum::{Extension, Router};
use notifico_subscription::http::get_router as subscription_get_router;

pub(crate) fn get_router(ext: HttpExtensions) -> Router {
    Router::new()
        .nest("/", subscription_get_router(ext.subman.clone()))
        .layer(Extension(ext.subman.clone()))
}
