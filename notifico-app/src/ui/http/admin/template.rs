use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{AdminCrudTable, ItemWithId, ListQueryParams};
use notifico_template::source::db::DbTemplateSource;
use notifico_template::source::TemplateItem;
use std::sync::Arc;
use uuid::Uuid;

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<DbTemplateSource>>,
) -> impl IntoResponse {
    controller.list(params).await.unwrap()
}

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

pub async fn create(
    Extension(controller): Extension<Arc<DbTemplateSource>>,
    Json(update): Json<TemplateItem>,
) -> impl IntoResponse {
    let result = controller.create(update).await.unwrap();
    (StatusCode::CREATED, Json(result))
}

pub async fn update(
    Extension(controller): Extension<Arc<DbTemplateSource>>,
    Json(update): Json<ItemWithId<TemplateItem>>,
) -> impl IntoResponse {
    let result = controller.update(update.id, update.item).await.unwrap();
    (StatusCode::ACCEPTED, Json(result))
}

pub async fn delete(
    Extension(controller): Extension<Arc<DbTemplateSource>>,
    Path((_channel, id)): Path<(String, Uuid)>,
) -> impl IntoResponse {
    controller.delete(id).await.unwrap();
    StatusCode::NO_CONTENT
}
