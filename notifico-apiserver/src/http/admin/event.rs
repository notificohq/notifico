use axum::extract::Query;
use axum::http::header::CONTENT_RANGE;
use axum::http::HeaderMap;
use axum::{Extension, Json};
use notifico_core::http::admin::ListQueryParams;
use notifico_core::pipeline::storage::PipelineStorage;
use notifico_core::pipeline::Event;
use std::sync::Arc;

pub async fn list_events(
    Query(params): Query<ListQueryParams>,
    Extension(pipeline_storage): Extension<Arc<dyn PipelineStorage>>,
) -> (HeaderMap, Json<Vec<Event>>) {
    let (query_result, count) = pipeline_storage.list_events(params).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, count.into());

    (headers, Json(query_result))
}
