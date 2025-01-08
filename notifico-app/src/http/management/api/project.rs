use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ReactAdminListQueryParams, RefineListQueryParams,
};
use notifico_project::{Project, ProjectController};
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/v1/projects",
    params(ReactAdminListQueryParams, RefineListQueryParams)
)]
pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<ProjectController>>,
) -> impl IntoResponse {
    controller.list(params).await.unwrap()
}

#[utoipa::path(get, path = "/v1/projects/{id}")]
pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<ProjectController>>,
) -> impl IntoResponse {
    let result = controller.get_by_id(id).await.unwrap();

    let Some(result) = result else {
        return (StatusCode::NOT_FOUND, Json(None));
    };
    (StatusCode::OK, Json(Some(ItemWithId { item: result, id })))
}

#[utoipa::path(post, path = "/v1/projects")]
pub async fn create(
    Extension(controller): Extension<Arc<ProjectController>>,
    Json(update): Json<Project>,
) -> impl IntoResponse {
    let result = controller.create(update).await.unwrap();
    (StatusCode::CREATED, Json(result))
}

#[utoipa::path(method(put, patch), path = "/v1/projects/{id}")]
pub async fn update(
    Extension(controller): Extension<Arc<ProjectController>>,
    Path((id,)): Path<(Uuid,)>,
    Json(update): Json<Project>,
) -> impl IntoResponse {
    let result = controller.update(id, update).await.unwrap();
    (StatusCode::ACCEPTED, Json(result))
}

#[utoipa::path(delete, path = "/v1/projects/{id}")]
pub async fn delete(
    Extension(controller): Extension<Arc<ProjectController>>,
    Path((id,)): Path<(Uuid,)>,
) -> impl IntoResponse {
    controller.delete(id).await.unwrap();
    StatusCode::NO_CONTENT
}
