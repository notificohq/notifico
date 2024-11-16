use axum::extract::{Path, Query};
use axum::http::header::CONTENT_RANGE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{ListQueryParams, PaginatedResult};
use notifico_project::ProjectController;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub async fn list_projects(
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<ProjectController>>,
) -> impl IntoResponse {
    let PaginatedResult { items, total_count } = controller.list(params).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, total_count.into());

    (headers, Json(items))
}

pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<ProjectController>>,
) -> impl IntoResponse {
    let result = controller.get_by_id(id).await.unwrap();

    let Some(result) = result else {
        return (StatusCode::NOT_FOUND, Json(None));
    };
    (StatusCode::OK, Json(Some(result)))
}

#[derive(Deserialize)]
pub struct ProjectUpdate {
    name: String,
}

pub async fn create(
    Extension(controller): Extension<Arc<ProjectController>>,
    Json(update): Json<ProjectUpdate>,
) -> impl IntoResponse {
    let result = controller.create(&update.name).await.unwrap();

    (
        StatusCode::CREATED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

pub async fn update(
    Extension(controller): Extension<Arc<ProjectController>>,
    Path((id,)): Path<(Uuid,)>,
    Json(update): Json<ProjectUpdate>,
) -> impl IntoResponse {
    let result = controller.update(id, &update.name).await.unwrap();

    (
        StatusCode::ACCEPTED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

pub async fn delete(
    Extension(controller): Extension<Arc<ProjectController>>,
    Path((id,)): Path<(Uuid,)>,
) -> impl IntoResponse {
    controller.delete(id).await.unwrap();

    (StatusCode::NO_CONTENT, Json(json!({})))
}
