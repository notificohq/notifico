use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{AdminCrudTable, ItemWithId, ListQueryParams};
use notifico_core::pipeline::Pipeline;
use notifico_dbpipeline::controllers::pipeline::{PipelineDbController, PipelineItem};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct RestPipelineItem {
    project_id: Uuid,
    steps: String,
    event_ids: Vec<Uuid>,
}

impl From<PipelineItem> for RestPipelineItem {
    fn from(value: PipelineItem) -> Self {
        Self {
            project_id: value.pipeline.project_id,
            steps: serde_json::to_string_pretty(&value.pipeline.steps).unwrap(),
            event_ids: value.event_ids,
        }
    }
}

impl From<RestPipelineItem> for PipelineItem {
    fn from(value: RestPipelineItem) -> Self {
        Self {
            pipeline: Pipeline {
                project_id: value.project_id,
                steps: serde_json::from_str(&value.steps).unwrap(),
            },
            event_ids: value.event_ids,
        }
    }
}

pub async fn create(
    Extension(pipeline_storage): Extension<Arc<PipelineDbController>>,
    Json(item): Json<RestPipelineItem>,
) -> impl IntoResponse {
    let result = pipeline_storage.create(item.into()).await.unwrap();
    (StatusCode::CREATED, Json(result))
}

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(pipeline_storage): Extension<Arc<PipelineDbController>>,
) -> impl IntoResponse {
    pipeline_storage
        .list(params)
        .await
        .unwrap()
        .map(|item| item.map(RestPipelineItem::from))
}

pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(pipeline_storage): Extension<Arc<PipelineDbController>>,
) -> impl IntoResponse {
    let result = pipeline_storage.get_by_id(id).await.unwrap();
    let Some(result) = result else {
        return (StatusCode::NOT_FOUND, Json(None));
    };
    (
        StatusCode::OK,
        Json(Some(ItemWithId {
            id,
            item: RestPipelineItem::from(result),
        })),
    )
}

pub async fn update(
    Extension(pipeline_storage): Extension<Arc<PipelineDbController>>,
    Path((id,)): Path<(Uuid,)>,
    Json(update): Json<RestPipelineItem>,
) -> impl IntoResponse {
    let result = pipeline_storage.update(id, update.into()).await.unwrap();
    (StatusCode::ACCEPTED, Json(result))
}

pub async fn delete(
    Extension(pipeline_storage): Extension<Arc<PipelineDbController>>,
    Path((id,)): Path<(Uuid,)>,
) -> impl IntoResponse {
    pipeline_storage.delete(id).await.unwrap();
    StatusCode::NO_CONTENT
}
