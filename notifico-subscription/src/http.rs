use crate::SubscriptionManager;
use axum::extract::Query;
use axum::routing::get;
use axum::{middleware, Extension, Router};
use notifico_core::http::auth::{authorize, Scope};
use notifico_core::http::AuthorizedRecipient;
use serde::Deserialize;
use std::sync::Arc;

pub fn get_router<S: Clone + Send + Sync + 'static>(
    ncenter: Arc<SubscriptionManager>,
) -> Router<S> {
    Router::new()
        .route("/v1/list_unsubscribe", get(list_unsubscribe))
        .layer(middleware::from_fn(authorize))
        .layer(Extension(Arc::new(Scope("list_unsubscribe".to_string()))))
        .layer(Extension(ncenter))
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    event: String,
}

const CHANNEL_EMAIL: &str = "email";

#[allow(private_interfaces)]
pub(crate) async fn list_unsubscribe(
    Query(params): Query<QueryParams>,
    Extension(sub_manager): Extension<Arc<SubscriptionManager>>,
    Extension(auth): Extension<Arc<AuthorizedRecipient>>,
) {
    sub_manager
        .set_subscribed(
            auth.project_id,
            auth.recipient_id,
            &params.event,
            CHANNEL_EMAIL,
            false,
        )
        .await;
}
