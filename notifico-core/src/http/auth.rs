use crate::http::{AuthorizedRecipient, SecretKey};
use axum::body::Body;
use axum::extract::{Query, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{http, Extension, Json};
use jwt::{Header, Token, VerifyWithKey};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct AuthError {
    message: String,
    status_code: StatusCode,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response<Body> {
        let body = Json(json!({
            "error": self.message,
        }));

        (self.status_code, body).into_response()
    }
}

#[derive(Clone, Deserialize)]
pub struct QueryParams {
    token: Option<String>,
}

/// Extracts the token from the query parameters or from Authorization header.
pub async fn authorize(
    Query(params): Query<QueryParams>,
    Extension(skey): Extension<Arc<SecretKey>>,
    mut req: Request,
    next: Next,
) -> Result<Response<Body>, AuthError> {
    let auth_header = req.headers_mut().get(http::header::AUTHORIZATION);

    let token = match (params.token, auth_header) {
        (Some(query_token), _) => query_token.clone(),
        (_, Some(auth_header)) => {
            let value = auth_header.to_str().map_err(|_| AuthError {
                message: "Empty header is not allowed".to_string(),
                status_code: StatusCode::FORBIDDEN,
            })?;

            let mut header = value.split_whitespace();
            let (_bearer, token) = (header.next(), header.next());

            let Some(token) = token else {
                return Err(AuthError {
                    message: "Missing bearer token".to_string(),
                    status_code: StatusCode::FORBIDDEN,
                });
            };
            token.to_string()
        }
        (None, None) => {
            return Err(AuthError {
                message: "No JWT token provided".to_string(),
                status_code: StatusCode::FORBIDDEN,
            })
        }
    };

    let token: Token<Header, BTreeMap<String, String>, _> = token.verify_with_key(&skey.0).unwrap();

    let claims = token.claims();
    let (Some(recipient_id), Some(project_id)) = (claims.get("sub"), claims.get("proj")) else {
        return Err(AuthError {
            status_code: StatusCode::FORBIDDEN,
            message: "Invalid JWT claims".to_string(),
        });
    };

    let recipient = AuthorizedRecipient {
        project_id: project_id.parse::<Uuid>().unwrap(),
        recipient_id: recipient_id.parse::<Uuid>().unwrap(),
    };

    req.extensions_mut().insert(Arc::new(recipient));
    Ok(next.run(req).await)
}
