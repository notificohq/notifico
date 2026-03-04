use std::sync::Arc;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::AppState;

/// Authenticated context extracted from API key.
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub project_id: Uuid,
    pub api_key_id: Uuid,
    pub scope: String,
}

/// Error returned when authentication fails.
#[derive(Debug)]
pub enum AuthError {
    MissingHeader,
    InvalidFormat,
    InvalidKey,
    DisabledKey,
    InsufficientScope { required: String, actual: String },
    DbError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingHeader => (StatusCode::UNAUTHORIZED, "Missing Authorization header"),
            AuthError::InvalidFormat => {
                (StatusCode::UNAUTHORIZED, "Invalid Authorization format, expected: Bearer <key>")
            }
            AuthError::InvalidKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
            AuthError::DisabledKey => (StatusCode::UNAUTHORIZED, "API key is disabled"),
            AuthError::InsufficientScope { .. } => {
                (StatusCode::FORBIDDEN, "Insufficient API key scope")
            }
            AuthError::DbError(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        (status, message).into_response()
    }
}

impl FromRequestParts<Arc<AppState>> for AuthContext {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        // Extract Bearer token
        let auth_header = parts
            .headers
            .get("authorization")
            .ok_or(AuthError::MissingHeader)?
            .to_str()
            .map_err(|_| AuthError::InvalidFormat)?;

        let raw_key = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidFormat)?;

        if raw_key.is_empty() {
            return Err(AuthError::InvalidFormat);
        }

        // Look up key in DB
        let key_info = notifico_db::repo::api_key::find_by_raw_key(&state.db, raw_key)
            .await
            .map_err(|e| AuthError::DbError(e.to_string()))?
            .ok_or(AuthError::InvalidKey)?;

        if !key_info.enabled {
            return Err(AuthError::DisabledKey);
        }

        Ok(AuthContext {
            project_id: key_info.project_id,
            api_key_id: key_info.id,
            scope: key_info.scope,
        })
    }
}

impl AuthContext {
    /// Verify the auth context has the required scope.
    pub fn require_scope(&self, required: &str) -> Result<(), AuthError> {
        if self.scope == required || self.scope == "admin" {
            Ok(())
        } else {
            Err(AuthError::InsufficientScope {
                required: required.to_string(),
                actual: self.scope.clone(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_scope_exact_match() {
        let ctx = AuthContext {
            project_id: Uuid::now_v7(),
            api_key_id: Uuid::now_v7(),
            scope: "ingest".into(),
        };
        assert!(ctx.require_scope("ingest").is_ok());
        assert!(ctx.require_scope("admin").is_err());
    }

    #[test]
    fn require_scope_admin_grants_all() {
        let ctx = AuthContext {
            project_id: Uuid::now_v7(),
            api_key_id: Uuid::now_v7(),
            scope: "admin".into(),
        };
        assert!(ctx.require_scope("ingest").is_ok());
        assert!(ctx.require_scope("public").is_ok());
        assert!(ctx.require_scope("admin").is_ok());
    }

    #[test]
    fn require_scope_mismatch() {
        let ctx = AuthContext {
            project_id: Uuid::now_v7(),
            api_key_id: Uuid::now_v7(),
            scope: "public".into(),
        };
        assert!(ctx.require_scope("ingest").is_err());
    }

    #[test]
    fn auth_error_status_codes() {
        let resp = AuthError::MissingHeader.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

        let resp = AuthError::InsufficientScope {
            required: "ingest".into(),
            actual: "public".into(),
        }
        .into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }
}
