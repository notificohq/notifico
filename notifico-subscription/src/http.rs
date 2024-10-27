use crate::SubscriptionManager;
use axum::extract::Query;
use axum::routing::get;
use axum::{Extension, Router};
use notifico_core::http::AuthorizedRecipient;
use serde::Deserialize;
use std::sync::Arc;

pub fn get_router<S: Clone + Send + Sync + 'static>(
    ncenter: Arc<SubscriptionManager>,
) -> Router<S> {
    Router::new()
        .route("/v1/list-unsubscribe", get(list_unsubscribe))
        .layer(Extension(ncenter))
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    event: String,
    channel: String,
}

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
            &params.channel,
            false,
        )
        .await;
}
