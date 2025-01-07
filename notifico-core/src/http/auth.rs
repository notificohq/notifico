use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

pub struct AuthError {
    message: String,
    status_code: StatusCode,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
#[serde(tag = "scope", rename_all = "kebab-case")]
pub enum Claims {
    /// This option limits token acceptability only for list-unsubscribe purposes, because
    /// Recipient is not authorized by the external system backend here.
    ListUnsubscribe {
        #[serde(rename = "evt")]
        event: String,
        #[serde(rename = "sub")]
        recipient_id: Uuid,
        exp: u64,
    },
    /// This claim is issued using server-to-server call to Management API.
    /// The external system's backend should validate the recipient's identity prior to issuing this
    /// token.
    /// This token can be used for changing recipient notification settings,
    /// reading web notifications, connecting to websocket for real-time notifications, etc.
    General {
        #[serde(rename = "sub")]
        recipient_id: Uuid,
        exp: u64,
    },
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response<Body> {
        let body = Json(json!({
            "error": self.message,
        }));

        (self.status_code, body).into_response()
    }
}
