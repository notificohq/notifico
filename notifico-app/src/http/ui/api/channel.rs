use axum::response::IntoResponse;
use axum::{Extension, Json};
use notifico_core::transport::TransportRegistry;
use std::sync::Arc;

#[utoipa::path(get, path = "/v1/channels")]
pub async fn list(Extension(controller): Extension<Arc<TransportRegistry>>) -> impl IntoResponse {
    Json(controller.supported_channels().clone())
}
