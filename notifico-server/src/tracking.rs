use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{StatusCode, header},
    response::{IntoResponse, Redirect, Response},
};
use base64::engine::{Engine, general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

/// 1x1 transparent GIF (43 bytes).
const TRANSPARENT_GIF: [u8; 43] = [
    0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, 0xff, 0xff,
    0xff, 0x00, 0x00, 0x00, 0x21, 0xf9, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00,
    0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44, 0x01, 0x00, 0x3b,
];

/// Create a signed tracking token.
///
/// Token format: `base64url(json_payload).base64url(hmac_signature)`
pub fn create_tracking_token(delivery_log_id: &str, url: Option<&str>, key: &[u8; 32]) -> String {
    let payload = match url {
        Some(u) => serde_json::json!({"d": delivery_log_id, "u": u}),
        None => serde_json::json!({"d": delivery_log_id}),
    };
    let payload_bytes = serde_json::to_vec(&payload).expect("JSON serialization cannot fail");
    let encoded_payload = URL_SAFE_NO_PAD.encode(&payload_bytes);

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length is always valid");
    mac.update(encoded_payload.as_bytes());
    let signature = mac.finalize().into_bytes();
    let encoded_sig = URL_SAFE_NO_PAD.encode(signature);

    format!("{encoded_payload}.{encoded_sig}")
}

/// Verify a tracking token and return (delivery_log_id, optional_url).
pub fn verify_tracking_token(token: &str, key: &[u8; 32]) -> Option<(String, Option<String>)> {
    let (encoded_payload, encoded_sig) = token.split_once('.')?;

    let signature = URL_SAFE_NO_PAD.decode(encoded_sig).ok()?;

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key length is always valid");
    mac.update(encoded_payload.as_bytes());
    mac.verify_slice(&signature).ok()?;

    let payload_bytes = URL_SAFE_NO_PAD.decode(encoded_payload).ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;

    let delivery_log_id = payload.get("d")?.as_str()?.to_string();
    let url = payload
        .get("u")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some((delivery_log_id, url))
}

/// Handle open-tracking pixel requests.
///
/// Always returns the 1x1 GIF regardless of token validity (don't leak info).
pub async fn handle_open(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Response {
    if let Some(key) = &state.encryption_key {
        if let Some((delivery_log_id, _)) = verify_tracking_token(&token, key) {
            let id = Uuid::now_v7();
            let _ = notifico_db::repo::tracking::insert_tracking_event(
                &state.db,
                id,
                &delivery_log_id,
                "open",
                None,
            )
            .await;
        }
    }

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/gif")],
        TRANSPARENT_GIF.to_vec(),
    )
        .into_response()
}

/// Handle click-tracking redirect requests.
///
/// Returns 302 redirect on success, 400 on invalid token.
pub async fn handle_click(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Response {
    let key = match &state.encryption_key {
        Some(k) => k,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    let (delivery_log_id, url) = match verify_tracking_token(&token, key) {
        Some((d, Some(u))) => (d, u),
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let id = Uuid::now_v7();
    let _ = notifico_db::repo::tracking::insert_tracking_event(
        &state.db,
        id,
        &delivery_log_id,
        "click",
        Some(&url),
    )
    .await;

    Redirect::temporary(&url).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [0xAB; 32]
    }

    #[test]
    fn token_roundtrip_open() {
        let key = test_key();
        let token = create_tracking_token("delivery-123", None, &key);
        let result = verify_tracking_token(&token, &key);
        assert!(result.is_some());
        let (d, u) = result.unwrap();
        assert_eq!(d, "delivery-123");
        assert!(u.is_none());
    }

    #[test]
    fn token_roundtrip_click() {
        let key = test_key();
        let url = "https://example.com/landing?ref=abc";
        let token = create_tracking_token("delivery-456", Some(url), &key);
        let result = verify_tracking_token(&token, &key);
        assert!(result.is_some());
        let (d, u) = result.unwrap();
        assert_eq!(d, "delivery-456");
        assert_eq!(u.unwrap(), url);
    }

    #[test]
    fn tampered_token_fails() {
        let key = test_key();
        let token = create_tracking_token("delivery-123", None, &key);
        // Tamper with the payload
        let tampered = format!("x{token}");
        assert!(verify_tracking_token(&tampered, &key).is_none());
    }

    #[test]
    fn wrong_key_fails() {
        let key = test_key();
        let token = create_tracking_token("delivery-123", None, &key);
        let wrong_key = [0xCD; 32];
        assert!(verify_tracking_token(&token, &wrong_key).is_none());
    }

    #[test]
    fn missing_dot_fails() {
        let key = test_key();
        assert!(verify_tracking_token("nodothere", &key).is_none());
    }

    #[test]
    fn empty_token_fails() {
        let key = test_key();
        assert!(verify_tracking_token("", &key).is_none());
    }
}
