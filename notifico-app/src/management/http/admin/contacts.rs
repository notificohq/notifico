use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{AdminCrudTable, ItemWithId, ListQueryParams};
use notifico_subscription::controllers::contact::{ContactDbController, ContactItem};
use std::sync::Arc;
use uuid::Uuid;

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<ContactDbController>>,
) -> impl IntoResponse {
    controller.list(params).await.unwrap()
}

pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<ContactDbController>>,
) -> impl IntoResponse {
    let item = controller.get_by_id(id).await.unwrap();

    let Some(item) = item else {
        return (StatusCode::NOT_FOUND, Json(None));
    };
    (StatusCode::OK, Json(Some(ItemWithId { item, id })))
}

pub async fn create(
    Extension(controller): Extension<Arc<ContactDbController>>,
    Json(update): Json<ContactItem>,
) -> impl IntoResponse {
    let result = controller.create(update).await.unwrap();

    (StatusCode::CREATED, Json(result))
}

pub async fn update(
    Extension(controller): Extension<Arc<ContactDbController>>,
    Path((id,)): Path<(Uuid,)>,
    Json(update): Json<ContactItem>,
) -> impl IntoResponse {
    let result = controller.update(id, update).await.unwrap();
    (StatusCode::ACCEPTED, Json(result))
}

pub async fn delete(
    Path((id,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<ContactDbController>>,
) -> impl IntoResponse {
    controller.delete(id).await.unwrap();
    StatusCode::NO_CONTENT
}
