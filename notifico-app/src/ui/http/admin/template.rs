use axum::extract::{Path, Query};
use axum::http::header::CONTENT_RANGE;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{AdminCrudTable, ItemWithId, ListQueryParams, PaginatedResult};
use notifico_template::source::db::DbTemplateSource;
use notifico_template::source::TemplateItem;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

pub async fn list(
    Path((channel,)): Path<(String,)>,
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<DbTemplateSource>>,
) -> impl IntoResponse {
    let extras = HashMap::from([("channel".to_string(), channel)]);
    let PaginatedResult { items, total_count } = controller.list(params, extras).await.unwrap();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_RANGE, total_count.into());

    (headers, Json(items))
}

pub async fn get(
    Path((_channel, id)): Path<(String, Uuid)>,
    Extension(controller): Extension<Arc<DbTemplateSource>>,
) -> (StatusCode, Json<Option<TemplateItem>>) {
    match controller.get_by_id(id).await {
        Ok(Some(template)) => (StatusCode::OK, Json(Some(template))),
        Ok(None) => (StatusCode::NOT_FOUND, Json(None)),
        Err(e) => panic!("{:?}", e),
    }
}

pub async fn create(
    Extension(controller): Extension<Arc<DbTemplateSource>>,
    Json(update): Json<TemplateItem>,
) -> (StatusCode, Json<Value>) {
    let result = controller.create(update).await.unwrap();

    (
        StatusCode::CREATED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

pub async fn update(
    Extension(controller): Extension<Arc<DbTemplateSource>>,
    Json(update): Json<ItemWithId<TemplateItem>>,
) -> (StatusCode, Json<Value>) {
    let result = controller.update(update.id, update.item).await.unwrap();

    (
        StatusCode::ACCEPTED,
        Json(serde_json::to_value(result).unwrap()),
    )
}

pub async fn delete(
    Extension(controller): Extension<Arc<DbTemplateSource>>,
    Path((_channel, id)): Path<(String, Uuid)>,
) -> (StatusCode, Json<Value>) {
    controller.delete(id).await.unwrap();

    (StatusCode::NO_CONTENT, Json(Value::Null))
}
