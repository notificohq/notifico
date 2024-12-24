use super::list_unsubscribe::get_router as subscription_get_router;
use crate::public::http::HttpPublicExtensions;
use axum::{Extension, Router};

pub(crate) fn get_router(ext: HttpPublicExtensions) -> Router {
    Router::new()
        .nest(
            "/",
            subscription_get_router(ext.subscription_controller.clone()),
        )
        .layer(Extension(ext.subscription_controller.clone()))
}
