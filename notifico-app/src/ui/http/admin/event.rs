use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{AdminCrudTable, ItemWithId, ListQueryParams};
use notifico_dbpipeline::controllers::event::{Event, EventDbController};
use std::sync::Arc;
use uuid::Uuid;

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(pipeline_storage): Extension<Arc<EventDbController>>,
) -> impl IntoResponse {
    pipeline_storage.list(params).await.unwrap()
}

pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<EventDbController>>,
) -> impl IntoResponse {
    let result = controller.get_by_id(id).await.unwrap();

    let Some(item) = result else {
        return (StatusCode::NOT_FOUND, Json(None));
    };
    (StatusCode::OK, Json(Some(ItemWithId { id, item })))
}

pub async fn create(
    Extension(controller): Extension<Arc<EventDbController>>,
    Json(create): Json<Event>,
) -> impl IntoResponse {
    let result = controller.create(create).await.unwrap();
    (StatusCode::CREATED, Json(result))
}

pub async fn update(
    Extension(controller): Extension<Arc<EventDbController>>,
    Path((id,)): Path<(Uuid,)>,
    Json(update): Json<Event>,
) -> impl IntoResponse {
    let result = controller.update(id, update).await.unwrap();

    (StatusCode::CREATED, Json(result))
}

pub async fn delete(
    Extension(controller): Extension<Arc<EventDbController>>,
    Path((id,)): Path<(Uuid,)>,
) -> impl IntoResponse {
    controller.delete(id).await.unwrap();
    StatusCode::NO_CONTENT
}
