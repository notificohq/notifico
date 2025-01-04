use axum::extract::Query;
use axum::http::StatusCode;
use axum::Extension;
use jsonwebtoken::{DecodingKey, Validation};
use notifico_core::http::auth::Claims;
use notifico_core::http::SecretKey;
use notifico_subscription::controllers::subscription::SubscriptionDbController;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use utoipa::{IntoParams, OpenApi};
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

    let app = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(list_unsubscribe))
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
) -> StatusCode {
    let token = jsonwebtoken::decode::<Claims>(
        &params.token,
        &DecodingKey::from_secret(&secret_key.0),
        &Validation::default(),
    );

    let token = match token {
        Ok(token) => token,
        Err(_) => return StatusCode::FORBIDDEN,
    };

    match token.claims {
        Claims::ListUnsubscribe {
            event,
            recipient_id,
            ..
        } => {
            sub_manager
                .set_subscribed(recipient_id, &event, "email", false)
                .await
        }
    };
    // TODO: redirect to the success page (with a confirmation message and unsubscribe reasons)
    StatusCode::OK
}
