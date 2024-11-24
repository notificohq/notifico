use super::list_unsubscribe::get_router as subscription_get_router;
use crate::http::HttpExtensions;
use axum::{Extension, Router};

pub(crate) fn get_router(ext: HttpExtensions) -> Router {
    Router::new()
        .nest("/", subscription_get_router(ext.subman.clone()))
        .layer(Extension(ext.subman.clone()))
}
