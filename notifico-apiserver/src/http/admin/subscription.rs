use axum::extract::{Path, Query};
use axum::http::header::CONTENT_RANGE;
use axum::http::HeaderMap;
use axum::{Extension, Json};
use notifico_core::http::admin::ListQueryParams;
use notifico_subscription::SubscriptionManager;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Serialize)]
pub struct SubscriptionItem {
    pub id: Uuid,
    pub project_id: Uuid,
    pub recipient_id: Uuid,
    pub event: String,
    pub channel: String,
    pub is_subscribed: bool,
}

impl From<notifico_subscription::entity::subscription::Model> for SubscriptionItem {
    fn from(value: notifico_subscription::entity::subscription::Model) -> Self {
        SubscriptionItem {
            id: value.id,
            project_id: value.project_id,
            recipient_id: value.recipient_id,
            event: value.event,
            channel: value.channel,
            is_subscribed: value.is_subscribed,
        }
    }
}

pub async fn list_subscriptions(
    Query(params): Query<ListQueryParams>,
    Extension(subman): Extension<Arc<SubscriptionManager>>,
) -> (HeaderMap, Json<Vec<SubscriptionItem>>) {
    let (query_result, count) = subman.list_subscriptions(params).await.unwrap();

    let subscriptions = query_result
        .into_iter()
        .map(SubscriptionItem::from)
        .collect();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, count.into());

    (headers, Json(subscriptions))
}

pub async fn get_subscription(
    Path((params,)): Path<(Uuid,)>,
    Extension(subman): Extension<Arc<SubscriptionManager>>,
) -> Json<Value> {
    let result = subman.get_by_id(params).await.unwrap();

    let Some(result) = result else {
        return Json(json!({}));
    };
    Json(serde_json::to_value(SubscriptionItem::from(result)).unwrap())
}

#[derive(Clone, Deserialize)]
pub struct SubscriptionUpdate {
    pub is_subscribed: Option<bool>,
}

pub async fn update_subscription(
    Path((id,)): Path<(Uuid,)>,
    Extension(subman): Extension<Arc<SubscriptionManager>>,
    Json(update): Json<SubscriptionUpdate>,
) -> Json<SubscriptionItem> {
    subman
        .update_subscription(id, update.is_subscribed.unwrap())
        .await
        .unwrap();

    let result = subman.get_by_id(id).await.unwrap().unwrap();
    Json(result.into())
}
