use crate::http::SharedState;
use anyhow::anyhow;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use jwt::{Header, Token, VerifyWithKey};
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

pub(crate) struct JwtError(StatusCode, anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for JwtError {
    fn into_response(self) -> Response {
        (self.0, format!("Something went wrong: {}", self.1)).into_response()
    }
}

impl From<jwt::Error> for JwtError {
    fn from(value: jwt::Error) -> Self {
        Self(StatusCode::FORBIDDEN, value.into())
    }
}

impl From<uuid::Error> for JwtError {
    fn from(value: uuid::Error) -> Self {
        Self(StatusCode::FORBIDDEN, value.into())
    }
}

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    token: String,
    event: String,
    channel: String,
}

pub(crate) async fn list_unsubscribe(
    Query(params): Query<QueryParams>,
    State(state): State<SharedState>,
) -> Result<(), JwtError> {
    let token: Token<Header, BTreeMap<String, String>, _> =
        params.token.verify_with_key(&state.secret_key)?;

    let claims = token.claims();
    let (Some(recipient_id), Some(project_id)) = (claims.get("sub"), claims.get("proj")) else {
        return Err(JwtError(
            StatusCode::FORBIDDEN,
            anyhow!("Invalid JWT claims"),
        ));
    };

    let recipient_id = recipient_id.parse::<Uuid>()?;
    let project_id = project_id.parse::<Uuid>()?;

    state
        .sub_manager
        .unsubscribe(
            project_id,
            recipient_id,
            &params.event,
            &params.channel,
            false,
        )
        .await;
    Ok(())
}
