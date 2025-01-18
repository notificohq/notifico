use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, RefineListQueryParams,
};
use notifico_template::source::db::DbTemplateSource;
use notifico_template::source::db::TemplateItem;
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(get, path = "/v1/templates", params(RefineListQueryParams))]
pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<DbTemplateSource>>,
) -> impl IntoResponse {
    controller.list(params).await.unwrap()
}

#[utoipa::path(get, path = "/v1/templates/{id}")]
pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<DbTemplateSource>>,
) -> impl IntoResponse {
    match controller.get_by_id(id).await {
        Ok(Some(item)) => (StatusCode::OK, Json(Some(ItemWithId { item, id }))),
        Ok(None) => (StatusCode::NOT_FOUND, Json(None)),
        Err(e) => panic!("{:?}", e),
    }
}

#[utoipa::path(post, path = "/v1/templates")]
pub async fn create(
    Extension(controller): Extension<Arc<DbTemplateSource>>,
    Json(update): Json<TemplateItem>,
) -> impl IntoResponse {
    let result = controller.create(update).await.unwrap();
    (StatusCode::CREATED, Json(result))
}

#[utoipa::path(method(put, patch), path = "/v1/templates/{id}")]
pub async fn update(
    Extension(controller): Extension<Arc<DbTemplateSource>>,
    Json(update): Json<ItemWithId<TemplateItem>>,
) -> impl IntoResponse {
    let result = controller.update(update.id, update.item).await.unwrap();
    (StatusCode::ACCEPTED, Json(result))
}

#[utoipa::path(delete, path = "/v1/templates/{id}")]
pub async fn delete(
    Extension(controller): Extension<Arc<DbTemplateSource>>,
    Path((id,)): Path<(Uuid,)>,
) -> impl IntoResponse {
    controller.delete(id).await.unwrap();
    StatusCode::NO_CONTENT
}
