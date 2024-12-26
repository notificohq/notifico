use axum::extract::{Path, Query};
use axum::http::header::CONTENT_RANGE;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{AdminCrudTable, ListQueryParams, PaginatedResult};
use notifico_subscription::SubscriptionController;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(subman): Extension<Arc<SubscriptionController>>,
) -> impl IntoResponse {
    let PaginatedResult { items, total_count } =
        subman.list(params, Default::default()).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, total_count.into());

    (headers, Json(items))
}

pub async fn get(
    Path((params,)): Path<(Uuid,)>,
    Extension(subman): Extension<Arc<SubscriptionController>>,
) -> impl IntoResponse {
    let result = subman.get_by_id(params).await.unwrap();

    let Some(result) = result else {
        return Json(json!({}));
    };
    Json(serde_json::to_value(result).unwrap())
}
