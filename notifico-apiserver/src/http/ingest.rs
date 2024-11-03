use crate::http::HttpExtensions;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Extension, Json, Router};
use notifico_core::pipeline::runner::ProcessEventRequest;
use tokio::sync::mpsc::Sender;

pub(crate) fn get_router(ext: HttpExtensions) -> Router {
    Router::new()
        .route("/v1/event/send", post(send))
        .layer(Extension(ext.sender))
}

#[utoipa::path(post, path = "/api/v1/event/send")]
async fn send(
    Extension(sender): Extension<Sender<ProcessEventRequest>>,
    Json(payload): Json<ProcessEventRequest>,
) -> StatusCode {
    sender.send(payload).await.unwrap();

    StatusCode::CREATED
}
