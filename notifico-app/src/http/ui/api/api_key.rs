use crate::controllers::api_key::{ApiKey, ApiKeyController};
use crate::crud_table::{AdminCrudTable, ItemWithId, ListQueryParams, RefineListQueryParams};
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyCreateUpdate {
    pub description: String,
    pub project_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyRead {
    pub id: Uuid,
    pub key: String,
    pub description: String,
    pub project_id: Uuid,
    pub created_at: chrono::NaiveDateTime,
}

#[utoipa::path(get, path = "/v1/api_keys", params(RefineListQueryParams))]
pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<ApiKeyController>>,
) -> impl IntoResponse {
    controller
        .list(params)
        .await
        .map(|result| result.map(ApiKeyRead::from))
        .unwrap()
}

#[utoipa::path(get, path = "/v1/api_keys/{id}")]
pub async fn get(
    Path((id,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<ApiKeyController>>,
) -> impl IntoResponse {
    let result = controller.get_by_id(id).await.unwrap();

    let Some(result) = result else {
        return (StatusCode::NOT_FOUND, Json(None));
    };
    (
        StatusCode::OK,
        Json(Some(ApiKeyRead::from(ItemWithId { item: result, id }))),
    )
}

#[utoipa::path(post, path = "/v1/api_keys")]
pub async fn create(
    Extension(controller): Extension<Arc<ApiKeyController>>,
    Json(update): Json<ApiKeyCreateUpdate>,
) -> impl IntoResponse {
    let result = controller.create(update.into()).await.unwrap();
    (StatusCode::CREATED, Json(result))
}

#[utoipa::path(method(put, patch), path = "/v1/api_keys/{id}")]
pub async fn update(
    Extension(controller): Extension<Arc<ApiKeyController>>,
    Path((id,)): Path<(Uuid,)>,
    Json(update): Json<ApiKeyCreateUpdate>,
) -> impl IntoResponse {
    let result = controller.update(id, update.into()).await.unwrap();
    (StatusCode::ACCEPTED, Json(result))
}

#[utoipa::path(delete, path = "/v1/api_keys/{id}")]
pub async fn delete(
    Extension(controller): Extension<Arc<ApiKeyController>>,
    Path((id,)): Path<(Uuid,)>,
) -> impl IntoResponse {
    controller.delete(id).await.unwrap();
    StatusCode::NO_CONTENT
}

impl From<ApiKeyCreateUpdate> for ApiKey {
    fn from(value: ApiKeyCreateUpdate) -> Self {
        Self {
            key: Default::default(),
            description: value.description,
            project_id: value.project_id.unwrap_or(Uuid::nil()),
            created_at: None,
        }
    }
}

impl From<ItemWithId<ApiKey>> for ApiKeyRead {
    fn from(value: ItemWithId<ApiKey>) -> Self {
        ApiKeyRead {
            id: value.id,
            key: value.item.key.simple().to_string(),
            description: value.item.description,
            project_id: value.item.project_id,
            created_at: value.item.created_at.unwrap(),
        }
    }
}
