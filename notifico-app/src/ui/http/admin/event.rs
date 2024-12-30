use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::ListQueryParams;
use notifico_core::pipeline::storage::PipelineStorage;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
) -> impl IntoResponse {
    pipeline_storage.list_events(params).await.unwrap()
}

pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
) -> impl IntoResponse {
    let result = pipeline_storage.get_event_by_id(id).await.unwrap();

    let Some(result) = result else {
        return (StatusCode::NOT_FOUND, Json(None));
    };
    (StatusCode::OK, Json(Some(result)))
}

#[derive(Deserialize)]
pub struct EventCreate {
    project_id: Uuid,
    name: String,
}

pub async fn create(
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
    Json(create): Json<EventCreate>,
) -> impl IntoResponse {
    let result = pipeline_storage
        .create_event(create.project_id, &create.name)
        .await
        .unwrap();

    (
        StatusCode::CREATED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

#[derive(Deserialize)]
pub struct EventUpdate {
    name: String,
}

pub async fn update(
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
    Path((id,)): Path<(Uuid,)>,
    Json(update): Json<EventUpdate>,
) -> impl IntoResponse {
    let result = pipeline_storage
        .update_event(id, &update.name)
        .await
        .unwrap();

    (
        StatusCode::CREATED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

pub async fn delete(
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
    Path((id,)): Path<(Uuid,)>,
) -> impl IntoResponse {
    pipeline_storage.delete_event(id).await.unwrap();

    (StatusCode::NO_CONTENT, Json(json!({})))
}
