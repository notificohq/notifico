use axum::extract::{Path, Query};
use axum::http::header::CONTENT_RANGE;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use notifico_core::http::admin::{ListQueryParams, PaginatedResult};
use notifico_core::pipeline::storage::{PipelineResult, PipelineStorage};
use notifico_core::pipeline::Pipeline;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    #[serde(default = "Uuid::nil")]
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_ids: Vec<Uuid>,
    pub steps: String,
}

impl From<PipelineResult> for PipelineItem {
    fn from(value: PipelineResult) -> Self {
        Self {
            id: value.pipeline.id,
            project_id: value.pipeline.project_id,
            steps: serde_json::to_string_pretty(&value.pipeline.steps).unwrap(),

            event_ids: value.event_ids,
        }
    }
}

pub async fn create(
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
    Json(item): Json<PipelineItem>,
) -> (StatusCode, Json<PipelineItem>) {
    let id = Uuid::now_v7();
    let pipeline = Pipeline {
        id,
        project_id: item.project_id,
        steps: serde_json::from_str(&item.steps).unwrap(),
    };
    let _pipeline = pipeline_storage
        .create_pipeline(pipeline.clone())
        .await
        .unwrap();

    let pipelineresult = PipelineResult {
        pipeline,
        event_ids: item.event_ids.clone(),
    };

    pipeline_storage
        .assign_events_to_pipeline(id, item.event_ids.clone())
        .await
        .unwrap();

    (StatusCode::CREATED, Json(pipelineresult.into()))
}

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
) -> (HeaderMap, Json<Vec<PipelineItem>>) {
    let PaginatedResult { items, total_count } =
        pipeline_storage.list_pipelines(params).await.unwrap();

    let pipelines = items.into_iter().map(PipelineItem::from).collect();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, total_count.into());

    (headers, Json(pipelines))
}

pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
) -> (StatusCode, Json<Option<PipelineItem>>) {
    let result = pipeline_storage
        .get_pipeline_by_id(id)
        .await
        .unwrap()
        .map(PipelineItem::from);

    let Some(result) = result else {
        return (StatusCode::NOT_FOUND, Json(None));
    };
    (StatusCode::OK, Json(Some(result)))
}

pub async fn update(
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
    Path((id,)): Path<(Uuid,)>,
    Json(update): Json<PipelineItem>,
) -> (StatusCode, Json<Value>) {
    let pipeline = Pipeline {
        id,
        project_id: update.project_id,
        steps: serde_json::from_str(&update.steps).unwrap(),
    };
    pipeline_storage.update_pipeline(pipeline).await.unwrap();
    pipeline_storage
        .assign_events_to_pipeline(id, update.event_ids.clone())
        .await
        .unwrap();

    (
        StatusCode::ACCEPTED,
        Json(serde_json::to_value(()).unwrap()),
    )
}

pub async fn delete(
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
    Path((id,)): Path<(Uuid,)>,
) -> (StatusCode, Json<Value>) {
    pipeline_storage.delete_pipeline(id).await.unwrap();

    (StatusCode::NO_CONTENT, Json(Value::Null))
}
