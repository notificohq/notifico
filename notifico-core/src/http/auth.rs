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
#[serde(tag = "aud", rename_all = "kebab-case")]
pub enum Claims {
    ListUnsubscribe {
        #[serde(rename = "evt")]
        event: String,
        #[serde(rename = "proj")]
        project_id: Uuid,
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
