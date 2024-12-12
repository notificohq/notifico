use super::list_unsubscribe::get_router as subscription_get_router;
use crate::public::http::HttpUserapiExtensions;
use axum::{Extension, Router};

pub(crate) fn get_router(ext: HttpUserapiExtensions) -> Router {
    Router::new()
        .nest("/", subscription_get_router(ext.subman.clone()))
        .layer(Extension(ext.subman.clone()))
}
