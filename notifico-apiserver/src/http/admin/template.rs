use axum::extract::{Path, Query};
use axum::http::header::CONTENT_RANGE;
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use notifico_core::http::admin::{ListQueryParams, PaginatedResult};
use notifico_template::error::TemplaterError;
use notifico_template::source::{TemplateItem, TemplateSource};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

pub async fn list(
    Path((channel,)): Path<(String,)>,
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<dyn TemplateSource>>,
) -> (HeaderMap, Json<Vec<TemplateItem>>) {
    let PaginatedResult { items, total_count } =
        controller.list_templates(&channel, params).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, total_count.into());

    (headers, Json(items))
}

pub async fn get(
    Path((_channel, id)): Path<(String, Uuid)>,
    Extension(controller): Extension<Arc<dyn TemplateSource>>,
) -> (StatusCode, Json<Option<TemplateItem>>) {
    match controller.get_template_by_id(id).await {
        Ok(template) => (StatusCode::OK, Json(Some(template))),
        Err(TemplaterError::TemplateNotFound) => (StatusCode::NOT_FOUND, Json(None)),
        Err(e) => panic!("{:?}", e),
    }
}

pub async fn create(
    Extension(controller): Extension<Arc<dyn TemplateSource>>,
    Json(update): Json<TemplateItem>,
) -> (StatusCode, Json<Value>) {
    let result = controller.create_template(update).await.unwrap();

    (
        StatusCode::CREATED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

pub async fn update(
    Extension(controller): Extension<Arc<dyn TemplateSource>>,
    Json(update): Json<TemplateItem>,
) -> (StatusCode, Json<Value>) {
    let result = controller.update_template(update).await.unwrap();

    (
        StatusCode::ACCEPTED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

pub async fn delete(
    Extension(controller): Extension<Arc<dyn TemplateSource>>,
    Path((_channel, id)): Path<(String, Uuid)>,
) -> (StatusCode, Json<Value>) {
    controller.delete_template(id).await.unwrap();

    (StatusCode::NO_CONTENT, Json(Value::Null))
}
