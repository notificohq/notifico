use axum::extract::{Path, Query};
use axum::http::header::CONTENT_RANGE;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use notifico_core::http::admin::{ListQueryParams, PaginatedResult};
use notifico_core::pipeline::storage::PipelineStorage;
use notifico_core::pipeline::Event;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
) -> (HeaderMap, Json<Vec<Event>>) {
    let PaginatedResult { items, total_count } =
        pipeline_storage.list_events(params).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, total_count.into());

    (headers, Json(items))
}

pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
) -> (StatusCode, Json<Option<Event>>) {
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
) -> (StatusCode, Json<Value>) {
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
) -> (StatusCode, Json<Value>) {
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
) -> (StatusCode, Json<Value>) {
    pipeline_storage.delete_event(id).await.unwrap();

    (StatusCode::NO_CONTENT, Json(json!({})))
}
