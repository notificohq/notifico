use crate::http::{AuthorizedRecipient, SecretKey};
use axum::body::Body;
use axum::extract::{Query, Request};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::{http, Extension, Json};
use jsonwebtoken::{DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

pub struct AuthError {
    message: String,
    status_code: StatusCode,
}

#[derive(Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename = "snake_case")]
pub enum Scopes {
    RecipientApi,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Claims {
    pub proj: Uuid,  // Project ID
    pub sub: String, // Recipient ID
    pub scopes: Vec<Scopes>,
    pub exp: usize,
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
    Extension(scope): Extension<Scopes>,
    mut req: Request,
    next: Next,
) -> Result<Response<Body>, AuthError> {
    let auth_header = req.headers_mut().get(http::header::AUTHORIZATION);

    // Extract token from query parameters or Authorization header
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

    let token = jsonwebtoken::decode::<Claims>(
        &token,
        &DecodingKey::from_secret(&skey.0),
        &Validation::default(),
    );

    let token = match token {
        Ok(token) => token,
        Err(_) => {
            return Err(AuthError {
                message: "Invalid JWT token".to_string(),
                status_code: StatusCode::FORBIDDEN,
            })
        }
    };

    // Check scopes
    if !token.claims.scopes.contains(&scope) {
        return Err(AuthError {
            message: "Insufficient scopes".to_string(),
            status_code: StatusCode::FORBIDDEN,
        });
    }

    let project_id = token.claims.proj;
    let recipient_id = token.claims.sub;

    let recipient = AuthorizedRecipient {
        project_id,
        recipient_id,
    };

    req.extensions_mut().insert(Arc::new(recipient));
    Ok(next.run(req).await)
}
