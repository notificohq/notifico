use crate::http::management::HttpManagementExtensions;
use axum::{Extension, Router};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::{Config, SwaggerUi};

mod contacts;
mod event;
mod group;
mod pipeline;
mod project;
mod recipients;
pub mod subscription;
mod template;

#[derive(OpenApi)]
#[openapi(info(title = "Notifico Management API", version = "0.1.0"))]
struct ApiDoc;

pub(crate) fn get_router(ext: HttpManagementExtensions) -> Router {
    let router = OpenApiRouter::with_openapi(ApiDoc::openapi())
        // Subscriptions
        .routes(routes!(subscription::list))
        .routes(routes!(subscription::get))
        // Recipients
        .routes(routes!(recipients::list, recipients::create))
        .routes(routes!(
            recipients::get,
            recipients::update,
            recipients::delete
        ))
        .routes(routes!(recipients::token))
        // Contacts
        .routes(routes!(contacts::list, contacts::create))
        .routes(routes!(contacts::get, contacts::update, contacts::delete))
        // Groups
        .routes(routes!(group::list, group::create))
        .routes(routes!(group::get, group::update, group::delete))
        // Pipelines
        .routes(routes!(pipeline::list, pipeline::create))
        .routes(routes!(pipeline::get, pipeline::update, pipeline::delete))
        // Events
        .routes(routes!(event::list, event::create))
        .routes(routes!(event::get, event::update, event::delete))
        // Projects
        .routes(routes!(project::list, project::create))
        .routes(routes!(project::get, project::update, project::delete))
        // Templates
        .routes(routes!(template::list, template::create))
        .routes(routes!(template::get, template::update, template::delete))
        // Layers
        .layer(Extension(ext.recipient_controller))
        .layer(Extension(ext.subscription_controller))
        .layer(Extension(ext.pipeline_controller))
        .layer(Extension(ext.project_controller))
        .layer(Extension(ext.template_controller))
        .layer(Extension(ext.contact_controller))
        .layer(Extension(ext.event_controller))
        .layer(Extension(ext.group_controller))
        .layer(Extension(ext.secret_key))
        .layer(CorsLayer::permissive());

    let (router, api) = router.split_for_parts();

    let config = Config::from("/api/api-docs/openapi.json");
    router.merge(
        SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", api)
            .config(config),
    )
}
