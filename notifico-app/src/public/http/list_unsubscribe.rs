use axum::extract::Query;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Extension, Router};
use jsonwebtoken::{DecodingKey, Validation};
use notifico_core::http::auth::Claims;
use notifico_core::http::SecretKey;
use notifico_subscription::controllers::subscription::SubscriptionDbController;
use serde::Deserialize;
use std::sync::Arc;

pub fn get_router(sub_manager: Arc<SubscriptionDbController>) -> Router {
    Router::new()
        .route(
            "/v1/list_unsubscribe",
            get(list_unsubscribe).post(list_unsubscribe),
        )
        .layer(Extension(sub_manager))
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    token: String,
}

#[allow(private_interfaces)]
pub(crate) async fn list_unsubscribe(
    Query(params): Query<QueryParams>,
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
    StatusCode::OK
}
