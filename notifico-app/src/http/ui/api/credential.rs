use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::credentials::env::EnvCredentialStorage;
use std::sync::Arc;

#[utoipa::path(get, path = "/v1/credentials")]
pub async fn list(
    Extension(controller): Extension<Arc<EnvCredentialStorage>>,
) -> impl IntoResponse {
    Json(controller.list())
}
