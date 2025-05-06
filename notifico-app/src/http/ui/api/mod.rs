use crate::http::ui::HttpUiExtensions;
use axum::{Extension, Router};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::{Config, SwaggerUi};

mod api_key;
mod event;
mod pipeline;
mod project;

#[derive(OpenApi)]
#[openapi(info(title = "Notifico UI API", version = "0.1.0"))]
struct ApiDoc;

pub(crate) fn get_router(ext: HttpUiExtensions) -> Router {
    let router = OpenApiRouter::with_openapi(ApiDoc::openapi())
        // API Keys
        .routes(routes!(api_key::list, api_key::create))
        .routes(routes!(api_key::get, api_key::update, api_key::delete))
        // Pipelines
        .routes(routes!(pipeline::list, pipeline::create))
        .routes(routes!(pipeline::get, pipeline::update, pipeline::delete))
        // Events
        .routes(routes!(event::list, event::create))
        .routes(routes!(event::get, event::update, event::delete))
        // Projects
        .routes(routes!(project::list, project::create))
        .routes(routes!(project::get, project::update, project::delete))
        // Layers
        .layer(Extension(ext.pipeline_controller))
        .layer(Extension(ext.project_controller))
        .layer(Extension(ext.event_controller))
        .layer(Extension(ext.api_key_controller))
        .layer(CorsLayer::permissive());

    let (router, api) = router.split_for_parts();

    let config = Config::from("/api/api-docs/openapi.json");
    router.merge(
        SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", api)
            .config(config),
    )
}
