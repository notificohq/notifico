use axum::extract::Query;
use axum::http::header::CONTENT_RANGE;
use axum::http::HeaderMap;
use axum::{Extension, Json};
use notifico_core::http::admin::ListQueryParams;
use notifico_core::pipeline::storage::PipelineStorage;
use notifico_core::step::SerializedStep;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Serialize)]
pub struct PipelineItem {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_ids: Vec<Uuid>,
    pub steps: Vec<SerializedStep>,
}

pub async fn list_pipelines(
    Query(params): Query<ListQueryParams>,
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
) -> (HeaderMap, Json<Vec<PipelineItem>>) {
    let (query_result, count) = pipeline_storage
        .list_pipelines_with_events(params)
        .await
        .unwrap();

    let pipelines = query_result
        .into_iter()
        .map(|(pipeline, events)| PipelineItem {
            id: pipeline.id,
            project_id: pipeline.project_id,
            event_ids: events.into_iter().map(|e| e.id).collect(),
            steps: pipeline.steps.clone(),
        });

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, count.into());

    (headers, Json(pipelines.collect()))
}
