use crate::ui::http::HttpUiExtensions;
use axum::routing::get;
use axum::{Extension, Router};
use tower_http::cors::CorsLayer;

mod event;
mod pipeline;
mod project;
pub mod subscription;
mod template;

pub(crate) fn get_router(ext: HttpUiExtensions) -> Router {
    Router::new()
        // Subscriptions
        .route("/v1/subscriptions", get(subscription::list))
        .route("/v1/subscriptions/:id", get(subscription::get))
        // Pipelines
        .route("/v1/pipelines", get(pipeline::list).post(pipeline::create))
        .route(
            "/v1/pipelines/:id",
            get(pipeline::get)
                .put(pipeline::update)
                .delete(pipeline::delete),
        )
        // Events
        .route("/v1/events", get(event::list).post(event::create))
        .route(
            "/v1/events/:id",
            get(event::get).put(event::update).delete(event::delete),
        )
        // Projects
        .route(
            "/v1/projects",
            get(project::list_projects).post(project::create),
        )
        .route(
            "/v1/projects/:id",
            get(project::get)
                .put(project::update)
                .delete(project::delete),
        )
        .route(
            "/v1/templates/:channel",
            get(template::list).post(template::create),
        )
        .route(
            "/v1/templates/:channel/:id",
            get(template::get)
                .put(template::update)
                .delete(template::delete),
        )
        // Layers
        .layer(Extension(ext.subscription_controller))
        .layer(Extension(ext.pipeline_storage))
        .layer(Extension(ext.projects_controller))
        .layer(Extension(ext.templates_controller))
        .layer(CorsLayer::permissive())
}
