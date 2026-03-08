use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::broadcast::{BroadcastRequest, BroadcastResponse};
use crate::ingest::IngestResponse;

/// OpenAPI documentation for the Notifico API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Notifico API",
        version = "0.1.0",
        description = "Self-hosted notification server with multi-channel delivery",
        license(name = "Apache-2.0"),
    ),
    paths(
        crate::ingest::handle_ingest,
        crate::broadcast::handle_broadcast,
    ),
    components(schemas(
        IngestResponse,
        BroadcastRequest,
        BroadcastResponse,
    )),
    tags(
        (name = "events", description = "Event ingestion"),
        (name = "broadcasts", description = "Broadcast sending"),
        (name = "admin", description = "Admin CRUD operations"),
        (name = "public", description = "Public API (preferences, unsubscribe)"),
    )
)]
pub struct ApiDoc;

pub fn swagger_ui_router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    SwaggerUi::new("/swagger-ui")
        .url("/api/openapi.json", ApiDoc::openapi())
        .into()
}
