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

pub async fn list_subscriptions(
    Query(params): Query<ListQueryParams>,
    Extension(subman): Extension<Arc<SubscriptionManager>>,
) -> (HeaderMap, Json<Vec<SubscriptionItem>>) {
    let (query_result, count) = subman.list_subscriptions(params).await.unwrap();

    let mut result = vec![];
    let subscriptions = query_result.into_iter();

    for subscription in subscriptions {
        result.push(SubscriptionItem {
            id: subscription.id,
            project_id: subscription.project_id,
            recipient_id: subscription.recipient_id,
            event: subscription.event,
            channel: subscription.channel,
            is_subscribed: subscription.is_subscribed,
        });
    }

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, count.into());

    (headers, Json(result))
}

pub async fn get_subscription(
    Path((params,)): Path<(Uuid,)>,
    Extension(subman): Extension<Arc<SubscriptionManager>>,
) -> Json<Value> {
    let result = subman.get_by_id(params).await.unwrap();

    let Some(result) = result else {
        return Json(json!({}));
    };
    Json(
        serde_json::to_value(SubscriptionItem {
            id: result.id,
            project_id: result.project_id,
            recipient_id: result.recipient_id,
            event: result.event,
            channel: result.channel,
            is_subscribed: result.is_subscribed,
        })
        .unwrap(),
    )
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
    Json(SubscriptionItem {
        id: result.id,
        project_id: result.project_id,
        recipient_id: result.recipient_id,
        event: result.event,
        channel: result.channel,
        is_subscribed: result.is_subscribed,
    })
}
