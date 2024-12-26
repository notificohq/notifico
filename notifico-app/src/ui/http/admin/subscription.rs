use axum::extract::{Path, Query};
use axum::http::header::CONTENT_RANGE;
use axum::http::HeaderMap;
use axum::{Extension, Json};
use notifico_core::http::admin::{AdminCrudTable, ListQueryParams, PaginatedResult};
use notifico_subscription::{SubscriptionController, SubscriptionItem};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(subman): Extension<Arc<SubscriptionController>>,
) -> (HeaderMap, Json<Vec<SubscriptionItem>>) {
    let PaginatedResult { items, total_count } = subman.list(params).await.unwrap();

    let subscriptions = items.into_iter().map(|(_, model)| model).collect();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, total_count.into());

    (headers, Json(subscriptions))
}

pub async fn get(
    Path((params,)): Path<(Uuid,)>,
    Extension(subman): Extension<Arc<SubscriptionController>>,
) -> Json<Value> {
    let result = subman.get_by_id(params).await.unwrap();

    let Some(result) = result else {
        return Json(json!({}));
    };
    Json(serde_json::to_value(result).unwrap())
}
