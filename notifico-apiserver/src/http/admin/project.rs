use axum::extract::{Path, Query};
use axum::http::header::CONTENT_RANGE;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use notifico_core::http::admin::{ListQueryParams, PaginatedResult};
use notifico_project::{Project, ProjectController};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

pub async fn list_projects(
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<ProjectController>>,
) -> (HeaderMap, Json<Vec<Project>>) {
    let PaginatedResult { items, total_count } = controller.list(params).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, total_count.into());

    (headers, Json(items))
}

pub async fn get_project(
    Path((id,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<ProjectController>>,
) -> (StatusCode, Json<Value>) {
    let result = controller.get_by_id(id).await.unwrap();

    let Some(result) = result else {
        return (StatusCode::NOT_FOUND, Json(json!({})));
    };
    (StatusCode::OK, Json(serde_json::to_value(result).unwrap()))
}

#[derive(Deserialize)]
pub struct ProjectUpdate {
    name: String,
}

pub async fn create_project(
    Extension(controller): Extension<Arc<ProjectController>>,
    Json(update): Json<ProjectUpdate>,
) -> (StatusCode, Json<Value>) {
    let result = controller.create(&update.name).await.unwrap();

    (
        StatusCode::CREATED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

pub async fn update_project(
    Extension(controller): Extension<Arc<ProjectController>>,
    Path((id,)): Path<(Uuid,)>,
    Json(update): Json<ProjectUpdate>,
) -> (StatusCode, Json<Value>) {
    let result = controller.update(id, &update.name).await.unwrap();

    (
        StatusCode::ACCEPTED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

pub async fn delete_project(
    Extension(controller): Extension<Arc<ProjectController>>,
    Path((id,)): Path<(Uuid,)>,
) -> (StatusCode, Json<Value>) {
    controller.delete(id).await.unwrap();

    (StatusCode::NO_CONTENT, Json(json!({})))
}
