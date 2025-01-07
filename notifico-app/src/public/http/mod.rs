use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use jsonwebtoken::{DecodingKey, Validation};
use notifico_core::http::auth::Claims;
use notifico_core::http::SecretKey;
use notifico_subscription::controllers::subscription::SubscriptionDbController;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{IntoParams, OpenApi, ToSchema};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub(crate) struct HttpPublicExtensions {
    pub subscription_controller: Arc<SubscriptionDbController>,
    pub secret_key: Arc<SecretKey>,
}

#[derive(OpenApi)]
#[openapi(info(title = "Notifico Public API"))]
struct ApiDoc;

pub(crate) async fn start(bind: SocketAddr, ext: HttpPublicExtensions) {
    // Bind everything now to catch any errors before spinning up the coroutines
    let listener = TcpListener::bind(bind).await.unwrap();

    let mut openapi = ApiDoc::openapi();
    openapi.components.as_mut().unwrap().add_security_scheme(
        "public_token",
        SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("JWT")
                .build(),
        ),
    );

    let app = OpenApiRouter::with_openapi(openapi)
        .routes(routes!(list_unsubscribe))
        .routes(routes!(subscription_parameters))
        .layer(Extension(ext.secret_key.clone()))
        .layer(Extension(ext.subscription_controller.clone()));

    let (app, api) = app.split_for_parts();
    let app = app.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api));

    tokio::spawn(async { axum::serve(listener, app).await.unwrap() });
}

#[derive(Debug, Deserialize, IntoParams)]
struct ListUnsubscribeParams {
    /// JWT token, containing `list-unsubscribe` claim with event and recipient ID
    token: String,
}

#[utoipa::path(get,
    path = "/v1/email/unsubscribe",
    params(ListUnsubscribeParams),
    responses(
        (status = StatusCode::OK, description = "User unsubscribed successfully"),
        (status = StatusCode::FORBIDDEN, description = "JWT token is invalid, expired or missing claim"),
    )
)]
async fn list_unsubscribe(
    Query(params): Query<ListUnsubscribeParams>,
    Extension(sub_manager): Extension<Arc<SubscriptionDbController>>,
    Extension(secret_key): Extension<Arc<SecretKey>>,
) -> impl IntoResponse {
    let token = jsonwebtoken::decode::<Claims>(
        &params.token,
        &DecodingKey::from_secret(&secret_key.0),
        &Validation::default(),
    );

    let token = match token {
        Ok(token) => token,
        Err(_) => return StatusCode::FORBIDDEN,
    };

    let Claims::ListUnsubscribe {
        event,
        recipient_id,
        ..
    } = token.claims
    else {
        return StatusCode::FORBIDDEN;
    };

    sub_manager
        .set_subscribed(recipient_id, &event, "email", false)
        .await;

    // TODO: redirect to the success page (with a confirmation message and unsubscribe reasons)
    StatusCode::OK
}

#[derive(Debug, Deserialize, ToSchema)]
struct EventSettings {
    channel: String,
    enabled: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
struct SubSettings {
    events: HashMap<String, EventSettings>,
}

#[utoipa::path(post, path = "/v1/subscription", security(("public_token" = ["general"])))]
async fn subscription_parameters(
    TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
    Extension(secret_key): Extension<Arc<SecretKey>>,
    Extension(sub_manager): Extension<Arc<SubscriptionDbController>>,
    Json(settings): Json<SubSettings>,
) -> impl IntoResponse {
    let token = jsonwebtoken::decode::<Claims>(
        auth_header.token(),
        &DecodingKey::from_secret(&secret_key.0),
        &Validation::default(),
    );

    let token = match token {
        Ok(token) => token,
        Err(_) => return StatusCode::FORBIDDEN,
    };

    let Claims::General { recipient_id, .. } = token.claims else {
        return StatusCode::FORBIDDEN;
    };

    for (event, event_settings) in &settings.events {
        sub_manager
            .set_subscribed(
                recipient_id,
                event,
                &event_settings.channel,
                event_settings.enabled,
            )
            .await;
    }

    StatusCode::OK
}
