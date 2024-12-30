use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::http::admin::{AdminCrudTable, ItemWithId, ListQueryParams};
use notifico_subscription::controllers::recipient::{RecipientDbController, RecipientItem};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct RecipientRestItem {
    project_id: Uuid,
    extras: String,
}

impl From<RecipientItem> for RecipientRestItem {
    fn from(value: RecipientItem) -> Self {
        RecipientRestItem {
            project_id: value.project_id,
            extras: serde_json::to_string(&value.extras).unwrap(),
        }
    }
}

impl From<RecipientRestItem> for RecipientItem {
    fn from(value: RecipientRestItem) -> Self {
        RecipientItem {
            project_id: value.project_id,
            extras: serde_json::from_str(&value.extras).unwrap(),
        }
    }
}

pub async fn list(
    Query(params): Query<ListQueryParams>,
    Extension(controller): Extension<Arc<RecipientDbController>>,
) -> impl IntoResponse {
    controller
        .list(params)
        .await
        .unwrap()
        .map(|item| ItemWithId {
            id: item.id,
            item: RecipientRestItem::from(item.item),
        })
        .into_response()
}

pub async fn get(
    Path((params,)): Path<(Uuid,)>,
    Extension(controller): Extension<Arc<RecipientDbController>>,
) -> impl IntoResponse {
    let result = controller.get_by_id(params).await.unwrap();

    let Some(result) = result else {
        return Json(json!({}));
    };
    Json(
        serde_json::to_value(ItemWithId {
            item: RecipientRestItem::from(result),
            id: params,
        })
        .unwrap(),
    )
}

pub async fn create(
    Extension(controller): Extension<Arc<RecipientDbController>>,
    Json(update): Json<RecipientRestItem>,
) -> impl IntoResponse {
    let result = controller.create(update.into()).await.unwrap();

    (StatusCode::CREATED, Json(result))
}
