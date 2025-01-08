use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ReactAdminListQueryParams, RefineListQueryParams,
};
use notifico_subscription::controllers::subscription::SubscriptionDbController;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/v1/subscriptions",
    params(ReactAdminListQueryParams, RefineListQueryParams)
)]
pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<SubscriptionDbController>>,
) -> impl IntoResponse {
    controller.list(params).await.unwrap()
}

#[utoipa::path(get, path = "/v1/subscriptions/{id}")]
pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<SubscriptionDbController>>,
) -> impl IntoResponse {
    match controller.get_by_id(id).await {
        Ok(Some(item)) => (StatusCode::OK, Json(Some(ItemWithId { item, id }))),
        Ok(None) => (StatusCode::NOT_FOUND, Json(None)),
        Err(e) => panic!("{:?}", e),
    }
}