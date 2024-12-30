use crate::ui::http::HttpUiExtensions;
use axum::routing::get;
use axum::{Extension, Router};
use tower_http::cors::CorsLayer;

mod contacts;
mod event;
mod pipeline;
mod project;
mod recipients;
pub mod subscription;
mod template;

pub(crate) fn get_router(ext: HttpUiExtensions) -> Router {
    Router::new()
        // Subscriptions
        .route("/v1/subscriptions", get(subscription::list))
        .route("/v1/subscriptions/:id", get(subscription::get))
        // Recipients
        .route(
            "/v1/recipients",
            get(recipients::list).post(recipients::create),
        )
        .route(
            "/v1/recipients/:id",
            get(recipients::get)
                .put(recipients::update)
                .delete(recipients::delete),
        )
        // Contacts
        .route("/v1/contacts", get(contacts::list).post(contacts::create))
        .route(
            "/v1/contacts/:id",
            get(contacts::get)
                .delete(contacts::delete)
                .put(contacts::update),
        )
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
        .route("/v1/projects", get(project::list).post(project::create))
        .route(
            "/v1/projects/:id",
            get(project::get)
                .put(project::update)
                .delete(project::delete),
        )
        .route("/v1/templates", get(template::list).post(template::create))
        .route(
            "/v1/templates/:id",
            get(template::get)
                .put(template::update)
                .delete(template::delete),
        )
        // Layers
        .layer(Extension(ext.recipient_controller))
        .layer(Extension(ext.subscription_controller))
        .layer(Extension(ext.pipeline_controller))
        .layer(Extension(ext.project_controller))
        .layer(Extension(ext.template_controller))
        .layer(Extension(ext.contact_controller))
        .layer(Extension(ext.event_controller))
        .layer(CorsLayer::permissive())
}
