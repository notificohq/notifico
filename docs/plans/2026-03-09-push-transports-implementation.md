# Push Transports + Transport Crate Extraction — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extract all 7 existing transports from `notifico-core` into separate library crates under `transports/`, then add 3 new push notification transports (FCM, APNs, Web Push).

**Architecture:** Each transport becomes its own Cargo workspace crate under `transports/<name>/`. `notifico-core` keeps the `Transport` trait, `TransportRegistry`, schemas, and `DeliveryResult`. `notifico-server` depends on all transport crates and registers them.

**Tech Stack:** Rust 2024, async-trait, reqwest, lettre, fcm-v1, a2, web-push

---

### Task 1: Extract ConsoleTransport into `transports/console/`

**Files:**
- Create: `transports/console/Cargo.toml`
- Create: `transports/console/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)
- Modify: `notifico-server/Cargo.toml` (add dependency)
- Modify: `notifico-server/src/main.rs` (update import)
- Modify: `notifico-core/src/transport/mod.rs` (remove ConsoleTransport, keep trait+types)

**Step 1: Create the crate**

`transports/console/Cargo.toml`:
```toml
[package]
name = "notifico-transport-console"
version = "0.1.0"
edition = "2024"

[dependencies]
notifico-core = { path = "../../notifico-core" }
async-trait.workspace = true
serde_json.workspace = true
tracing.workspace = true
```

`transports/console/src/lib.rs`: Copy the `ConsoleTransport` struct, its `Transport` impl, and the `#[cfg(test)] mod tests` block from `notifico-core/src/transport/mod.rs`. Add imports:
```rust
use async_trait::async_trait;
use notifico_core::transport::{
    ChannelId, ContentField, ContentFieldType, ContentSchema, CredentialSchema,
    DeliveryResult, RenderedMessage, Transport,
};
use notifico_core::error::CoreError;
```

**Step 2: Add to workspace**

In root `Cargo.toml`, add `"transports/console"` to `workspace.members`.

In `notifico-server/Cargo.toml`, add:
```toml
notifico-transport-console = { path = "../transports/console" }
```

**Step 3: Update notifico-server imports**

In `notifico-server/src/main.rs`, change:
```rust
// OLD:
use notifico_core::transport::ConsoleTransport;
// NEW:
use notifico_transport_console::ConsoleTransport;
```

**Step 4: Remove ConsoleTransport from notifico-core**

In `notifico-core/src/transport/mod.rs`, remove the `ConsoleTransport` struct, its `impl Transport`, and its tests. Keep all trait definitions, schema types, `TransportRegistry`, `RenderedMessage`, `DeliveryResult`, etc.

**Step 5: Verify**

Run: `cargo test -p notifico-transport-console`
Run: `cargo test -p notifico-server`

**Step 6: Commit**

```
feat: extract ConsoleTransport into transports/console crate
```

---

### Task 2: Extract EmailTransport into `transports/email/`

**Files:**
- Create: `transports/email/Cargo.toml`
- Create: `transports/email/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)
- Modify: `notifico-core/Cargo.toml` (remove lettre dependency)
- Modify: `notifico-core/src/transport/mod.rs` (remove `pub mod email;`)
- Delete: `notifico-core/src/transport/email.rs`
- Modify: `notifico-server/Cargo.toml`
- Modify: `notifico-server/src/main.rs`

**Step 1: Create the crate**

`transports/email/Cargo.toml`:
```toml
[package]
name = "notifico-transport-email"
version = "0.1.0"
edition = "2024"

[dependencies]
notifico-core = { path = "../../notifico-core" }
async-trait.workspace = true
serde_json.workspace = true
lettre.workspace = true
tracing.workspace = true
```

`transports/email/src/lib.rs`: Move the entire contents of `notifico-core/src/transport/email.rs`. Update imports to use `notifico_core::transport::*` and `notifico_core::error::CoreError`.

**Step 2: Add to workspace + server dependency**

Root `Cargo.toml`: add `"transports/email"` to members.
`notifico-server/Cargo.toml`: add `notifico-transport-email = { path = "../transports/email" }`.

**Step 3: Update imports in main.rs**

```rust
// OLD:
use notifico_core::transport::email::EmailTransport;
// NEW:
use notifico_transport_email::EmailTransport;
```

**Step 4: Clean up notifico-core**

Remove `pub mod email;` from `notifico-core/src/transport/mod.rs`.
Delete `notifico-core/src/transport/email.rs`.
Remove `lettre` from `notifico-core/Cargo.toml` dependencies.

**Step 5: Verify**

Run: `cargo test -p notifico-transport-email`
Run: `cargo test -p notifico-server`

**Step 6: Commit**

```
refactor: extract EmailTransport into transports/email crate
```

---

### Task 3: Extract Telegram, Slack, Discord transports

Same pattern as Task 2 for each. Do all three in one task since they're identical in structure (all use `reqwest::Client`).

**Crates to create:**
- `transports/telegram/` — crate `notifico-transport-telegram`
- `transports/slack/` — crate `notifico-transport-slack`
- `transports/discord/` — crate `notifico-transport-discord`

**Each `Cargo.toml`:**
```toml
[package]
name = "notifico-transport-<name>"
version = "0.1.0"
edition = "2024"

[dependencies]
notifico-core = { path = "../../notifico-core" }
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
reqwest.workspace = true
tracing.workspace = true
```

**Each `src/lib.rs`:** Move contents from `notifico-core/src/transport/<name>.rs`. Update imports.

**notifico-core cleanup:**
- Remove `pub mod telegram;`, `pub mod slack;`, `pub mod discord;` from `mod.rs`
- Delete the three `.rs` files

**notifico-server:**
- Add three dependencies to `Cargo.toml`
- Update imports in `main.rs`:
```rust
use notifico_transport_telegram::TelegramTransport;
use notifico_transport_slack::SlackTransport;
use notifico_transport_discord::DiscordTransport;
```

**Verify:**

Run: `cargo test -p notifico-transport-telegram -p notifico-transport-slack -p notifico-transport-discord`
Run: `cargo test -p notifico-server`

**Commit:**
```
refactor: extract Telegram, Slack, Discord transports into separate crates
```

---

### Task 4: Extract TwilioSms and Webhook transports

**Crates to create:**
- `transports/twilio-sms/` — crate `notifico-transport-twilio-sms`
- `transports/webhook/` — crate `notifico-transport-webhook`

**twilio-sms `Cargo.toml`:**
```toml
[package]
name = "notifico-transport-twilio-sms"
version = "0.1.0"
edition = "2024"

[dependencies]
notifico-core = { path = "../../notifico-core" }
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
reqwest.workspace = true
tracing.workspace = true
```

**webhook `Cargo.toml`:**
```toml
[package]
name = "notifico-transport-webhook"
version = "0.1.0"
edition = "2024"

[dependencies]
notifico-core = { path = "../../notifico-core" }
async-trait.workspace = true
serde_json.workspace = true
reqwest.workspace = true
hmac.workspace = true
sha2.workspace = true
hex.workspace = true
tracing.workspace = true
```

**notifico-core cleanup:**
- Remove `pub mod sms_twilio;`, `pub mod webhook;` from `mod.rs`
- Delete both `.rs` files
- Remove `hmac`, `sha2`, `hex` from `notifico-core/Cargo.toml` if no longer used

**notifico-server imports:**
```rust
use notifico_transport_twilio_sms::TwilioSmsTransport;
use notifico_transport_webhook::WebhookTransport;
```

**Verify:**

Run: `cargo test -p notifico-transport-twilio-sms -p notifico-transport-webhook`
Run: `cargo test -p notifico-server`

**Commit:**
```
refactor: extract Twilio SMS and Webhook transports into separate crates
```

---

### Task 5: Clean up notifico-core

After Tasks 1–4, `notifico-core/src/transport/mod.rs` should only contain:
- `ChannelId` type
- `Transport` trait
- `TransportRegistry`
- `ContentSchema`, `ContentField`, `ContentFieldType`
- `CredentialSchema`, `CredentialField`
- `RenderedMessage`, `Attachment`, `AttachmentDisposition`
- `DeliveryResult`
- Registry tests

**Step 1: Verify no transport modules remain**

`notifico-core/src/transport/mod.rs` should have no `pub mod` lines for transports.

**Step 2: Clean dependencies**

Remove from `notifico-core/Cargo.toml` any deps only used by transports:
- `lettre` (email)
- `reqwest` (all HTTP transports)
- `hmac`, `sha2`, `hex` (webhook)

Keep: `async-trait`, `serde`, `serde_json`, `tracing`, `uuid`, `chrono`, `notifico-template`, `base64`, `regex`, `html2text` (these are used by middleware and core).

Check if `reqwest` is used anywhere else in notifico-core (middleware might use it). If not, remove.

**Step 3: Verify full workspace**

Run: `cargo test --workspace`

All tests across all crates must pass.

**Step 4: Commit**

```
refactor: clean up notifico-core after transport extraction
```

---

### Task 6: Add FCM transport (`transports/fcm/`)

**Files:**
- Create: `transports/fcm/Cargo.toml`
- Create: `transports/fcm/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)
- Modify: `notifico-server/Cargo.toml`
- Modify: `notifico-server/src/main.rs`

**Step 1: Write tests first**

`transports/fcm/src/lib.rs` — tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fcm_channel_id() {
        let t = FcmTransport::new();
        assert_eq!(t.channel_id().as_str(), "push_fcm");
        assert_eq!(t.display_name(), "Push (FCM)");
    }

    #[test]
    fn fcm_content_schema() {
        let t = FcmTransport::new();
        let schema = t.content_schema();
        assert_eq!(schema.fields.len(), 5);
        let required: Vec<_> = schema.fields.iter().filter(|f| f.required).map(|f| f.name.as_str()).collect();
        assert!(required.contains(&"title"));
        assert!(required.contains(&"body"));
    }

    #[test]
    fn fcm_credential_schema() {
        let t = FcmTransport::new();
        let schema = t.credential_schema();
        assert_eq!(schema.fields.len(), 1);
        assert!(schema.fields[0].secret);
        assert_eq!(schema.fields[0].name, "service_account_json");
    }

    #[test]
    fn fcm_missing_credentials() {
        let t = FcmTransport::new();
        let msg = RenderedMessage {
            channel: ChannelId::new("push_fcm"),
            recipient_contact: "device-token-123".into(),
            content: serde_json::json!({"title": "Test", "body": "Hello"}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };
        let result = tokio::runtime::Runtime::new().unwrap().block_on(t.send(&msg));
        assert!(result.is_err() || matches!(result, Ok(DeliveryResult::Failed { .. })));
    }
}
```

**Step 2: Implement FcmTransport**

`transports/fcm/Cargo.toml`:
```toml
[package]
name = "notifico-transport-fcm"
version = "0.1.0"
edition = "2024"

[dependencies]
notifico-core = { path = "../../notifico-core" }
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
reqwest.workspace = true
tracing.workspace = true
jsonwebtoken = "9"
chrono.workspace = true

[dev-dependencies]
tokio.workspace = true
```

`transports/fcm/src/lib.rs`:
```rust
use async_trait::async_trait;
use notifico_core::error::CoreError;
use notifico_core::transport::{
    ChannelId, ContentField, ContentFieldType, ContentSchema,
    CredentialField, CredentialSchema, DeliveryResult, RenderedMessage, Transport,
};
use reqwest::Client;
use serde::Deserialize;

pub struct FcmTransport {
    client: Client,
}

impl FcmTransport {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }
}

impl Default for FcmTransport {
    fn default() -> Self { Self::new() }
}

/// Google service account JSON structure (relevant fields only).
#[derive(Deserialize)]
struct ServiceAccount {
    project_id: String,
    private_key: String,
    client_email: String,
    token_uri: String,
}

/// JWT claims for Google OAuth2.
#[derive(serde::Serialize)]
struct Claims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

/// Google OAuth2 token response.
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// FCM v1 API response.
#[derive(Deserialize)]
struct FcmResponse {
    name: Option<String>,
}

/// FCM v1 API error response.
#[derive(Deserialize)]
struct FcmErrorResponse {
    error: Option<FcmErrorDetail>,
}

#[derive(Deserialize)]
struct FcmErrorDetail {
    code: Option<u16>,
    message: Option<String>,
    status: Option<String>,
}

async fn get_access_token(client: &Client, sa: &ServiceAccount) -> Result<String, CoreError> {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        iss: sa.client_email.clone(),
        scope: "https://www.googleapis.com/auth/firebase.messaging".into(),
        aud: sa.token_uri.clone(),
        iat: now,
        exp: now + 3600,
    };

    let key = jsonwebtoken::EncodingKey::from_rsa_pem(sa.private_key.as_bytes())
        .map_err(|e| CoreError::Transport(format!("Invalid private key: {e}")))?;

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
        &claims,
        &key,
    )
    .map_err(|e| CoreError::Transport(format!("JWT encode error: {e}")))?;

    let resp = client
        .post(&sa.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &token),
        ])
        .send()
        .await
        .map_err(|e| CoreError::Transport(format!("Token request failed: {e}")))?;

    let token_resp: TokenResponse = resp
        .json()
        .await
        .map_err(|e| CoreError::Transport(format!("Token parse error: {e}")))?;

    Ok(token_resp.access_token)
}

#[async_trait]
impl Transport for FcmTransport {
    fn channel_id(&self) -> ChannelId { ChannelId::new("push_fcm") }
    fn display_name(&self) -> &str { "Push (FCM)" }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField { name: "title".into(), field_type: ContentFieldType::Text, required: true, description: "Notification title".into() },
                ContentField { name: "body".into(), field_type: ContentFieldType::Text, required: true, description: "Notification body".into() },
                ContentField { name: "image_url".into(), field_type: ContentFieldType::Text, required: false, description: "Image URL".into() },
                ContentField { name: "data".into(), field_type: ContentFieldType::Json, required: false, description: "Custom key-value payload".into() },
                ContentField { name: "click_action".into(), field_type: ContentFieldType::Text, required: false, description: "Intent/URL on tap".into() },
            ],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![
                CredentialField { name: "service_account_json".into(), required: true, secret: true, description: "Google service account JSON".into() },
            ],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        let sa_json = message.credentials.get("service_account_json")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing service_account_json".into()))?;

        let sa: ServiceAccount = serde_json::from_str(sa_json)
            .map_err(|e| CoreError::Transport(format!("Invalid service account JSON: {e}")))?;

        let access_token = get_access_token(&self.client, &sa).await?;

        let title = message.content.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let body = message.content.get("body").and_then(|v| v.as_str()).unwrap_or("");

        let mut notification = serde_json::json!({ "title": title, "body": body });
        if let Some(image) = message.content.get("image_url").and_then(|v| v.as_str()) {
            notification["image"] = serde_json::json!(image);
        }

        let mut fcm_message = serde_json::json!({
            "message": {
                "token": message.recipient_contact,
                "notification": notification,
            }
        });

        if let Some(data) = message.content.get("data") {
            if data.is_object() {
                fcm_message["message"]["data"] = data.clone();
            }
        }
        if let Some(action) = message.content.get("click_action").and_then(|v| v.as_str()) {
            fcm_message["message"]["webpush"] = serde_json::json!({
                "fcm_options": { "link": action }
            });
        }

        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            sa.project_id
        );

        let resp = self.client
            .post(&url)
            .bearer_auth(&access_token)
            .json(&fcm_message)
            .send()
            .await
            .map_err(|e| CoreError::Transport(format!("FCM request failed: {e}")))?;

        let status = resp.status();
        if status.is_success() {
            let fcm_resp: FcmResponse = resp.json().await.unwrap_or(FcmResponse { name: None });
            Ok(DeliveryResult::Delivered { provider_message_id: fcm_resp.name })
        } else {
            let retryable = status.as_u16() == 429 || status.is_server_error();
            let error_text = resp.text().await.unwrap_or_default();
            Ok(DeliveryResult::Failed { error: format!("FCM {status}: {error_text}"), retryable })
        }
    }
}
```

**Step 3: Register in server**

Add to `notifico-server/Cargo.toml`:
```toml
notifico-transport-fcm = { path = "../transports/fcm" }
```

In `main.rs`:
```rust
use notifico_transport_fcm::FcmTransport;
// ...
registry.register(Arc::new(FcmTransport::new()));
```

**Step 4: Verify**

Run: `cargo test -p notifico-transport-fcm`
Run: `cargo test -p notifico-server`

**Step 5: Commit**

```
feat: add FCM push transport (transports/fcm)
```

---

### Task 7: Add APNs transport (`transports/apns/`)

**Files:**
- Create: `transports/apns/Cargo.toml`
- Create: `transports/apns/src/lib.rs`
- Modify: `Cargo.toml`, `notifico-server/Cargo.toml`, `notifico-server/src/main.rs`

**Step 1: Write tests first**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apns_channel_id() {
        let t = ApnsTransport::new();
        assert_eq!(t.channel_id().as_str(), "push_apns");
        assert_eq!(t.display_name(), "Push (APNs)");
    }

    #[test]
    fn apns_content_schema() {
        let t = ApnsTransport::new();
        let schema = t.content_schema();
        assert_eq!(schema.fields.len(), 6);
        let required: Vec<_> = schema.fields.iter().filter(|f| f.required).map(|f| f.name.as_str()).collect();
        assert!(required.contains(&"title"));
        assert!(required.contains(&"body"));
    }

    #[test]
    fn apns_credential_schema() {
        let t = ApnsTransport::new();
        let schema = t.credential_schema();
        assert_eq!(schema.fields.len(), 4);
        let secret_fields: Vec<_> = schema.fields.iter().filter(|f| f.secret).map(|f| f.name.as_str()).collect();
        assert!(secret_fields.contains(&"private_key"));
    }

    #[test]
    fn apns_missing_credentials() {
        let t = ApnsTransport::new();
        let msg = RenderedMessage {
            channel: ChannelId::new("push_apns"),
            recipient_contact: "device-token-hex".into(),
            content: serde_json::json!({"title": "Test", "body": "Hello"}),
            credentials: serde_json::json!({}),
            attachments: vec![],
        };
        let result = tokio::runtime::Runtime::new().unwrap().block_on(t.send(&msg));
        assert!(result.is_err() || matches!(result, Ok(DeliveryResult::Failed { .. })));
    }
}
```

**Step 2: Implement ApnsTransport**

`transports/apns/Cargo.toml`:
```toml
[package]
name = "notifico-transport-apns"
version = "0.1.0"
edition = "2024"

[dependencies]
notifico-core = { path = "../../notifico-core" }
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
a2 = "0.10"
tracing.workspace = true

[dev-dependencies]
tokio.workspace = true
```

`transports/apns/src/lib.rs`:
```rust
use async_trait::async_trait;
use notifico_core::error::CoreError;
use notifico_core::transport::{
    ChannelId, ContentField, ContentFieldType, ContentSchema,
    CredentialField, CredentialSchema, DeliveryResult, RenderedMessage, Transport,
};
use a2::{
    Client, DefaultNotificationBuilder, NotificationBuilder, NotificationOptions,
    Priority, Endpoint,
};

pub struct ApnsTransport;

impl ApnsTransport {
    pub fn new() -> Self { Self }
}

impl Default for ApnsTransport {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl Transport for ApnsTransport {
    fn channel_id(&self) -> ChannelId { ChannelId::new("push_apns") }
    fn display_name(&self) -> &str { "Push (APNs)" }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField { name: "title".into(), field_type: ContentFieldType::Text, required: true, description: "Alert title".into() },
                ContentField { name: "body".into(), field_type: ContentFieldType::Text, required: true, description: "Alert body".into() },
                ContentField { name: "badge".into(), field_type: ContentFieldType::Text, required: false, description: "Badge count".into() },
                ContentField { name: "sound".into(), field_type: ContentFieldType::Text, required: false, description: "Sound name".into() },
                ContentField { name: "data".into(), field_type: ContentFieldType::Json, required: false, description: "Custom payload".into() },
                ContentField { name: "category".into(), field_type: ContentFieldType::Text, required: false, description: "Notification category for actions".into() },
            ],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![
                CredentialField { name: "team_id".into(), required: true, secret: false, description: "Apple Developer Team ID".into() },
                CredentialField { name: "key_id".into(), required: true, secret: false, description: "Key ID from .p8 file".into() },
                CredentialField { name: "private_key".into(), required: true, secret: true, description: ".p8 private key contents".into() },
                CredentialField { name: "environment".into(), required: true, secret: false, description: "production or sandbox".into() },
            ],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        let team_id = message.credentials.get("team_id").and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing team_id".into()))?;
        let key_id = message.credentials.get("key_id").and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing key_id".into()))?;
        let private_key = message.credentials.get("private_key").and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing private_key".into()))?;
        let environment = message.credentials.get("environment").and_then(|v| v.as_str()).unwrap_or("production");

        let endpoint = if environment == "sandbox" { Endpoint::Sandbox } else { Endpoint::Production };

        let client = Client::token(
            &mut private_key.as_bytes().to_vec().as_slice(),
            key_id,
            team_id,
            endpoint,
        ).map_err(|e| CoreError::Transport(format!("APNs client error: {e}")))?;

        let title = message.content.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let body_text = message.content.get("body").and_then(|v| v.as_str()).unwrap_or("");
        let sound = message.content.get("sound").and_then(|v| v.as_str()).unwrap_or("default");

        let mut builder = DefaultNotificationBuilder::new()
            .set_title(title)
            .set_body(body_text)
            .set_sound(sound);

        if let Some(badge_str) = message.content.get("badge").and_then(|v| v.as_str()) {
            if let Ok(badge) = badge_str.parse::<u32>() {
                builder = builder.set_badge(badge);
            }
        }
        if let Some(badge_num) = message.content.get("badge").and_then(|v| v.as_u64()) {
            builder = builder.set_badge(badge_num as u32);
        }

        let device_token = &message.recipient_contact;
        let options = NotificationOptions {
            apns_priority: Some(Priority::High),
            ..Default::default()
        };
        let payload = builder.build(device_token, options);

        match client.send(payload).await {
            Ok(response) => {
                let apns_id = response.apns_id.map(|id| id.to_string());
                Ok(DeliveryResult::Delivered { provider_message_id: apns_id })
            }
            Err(e) => {
                let error_str = format!("{e}");
                let retryable = !error_str.contains("BadDeviceToken")
                    && !error_str.contains("Unregistered")
                    && !error_str.contains("DeviceTokenNotForTopic");
                Ok(DeliveryResult::Failed { error: error_str, retryable })
            }
        }
    }
}
```

**Note:** The exact `a2` API may differ slightly by version. Verify against `a2` docs and adjust `Client::token` signature and builder methods accordingly. The structure and logic above is correct — adapt method names to the actual crate API.

**Step 3: Register in server**

`notifico-server/Cargo.toml`: add `notifico-transport-apns = { path = "../transports/apns" }`

`main.rs`:
```rust
use notifico_transport_apns::ApnsTransport;
// ...
registry.register(Arc::new(ApnsTransport::new()));
```

**Step 4: Verify**

Run: `cargo test -p notifico-transport-apns`
Run: `cargo test -p notifico-server`

**Step 5: Commit**

```
feat: add APNs push transport (transports/apns)
```

---

### Task 8: Add Web Push transport (`transports/web-push/`)

**Files:**
- Create: `transports/web-push/Cargo.toml`
- Create: `transports/web-push/src/lib.rs`
- Modify: `Cargo.toml`, `notifico-server/Cargo.toml`, `notifico-server/src/main.rs`

**Step 1: Write tests first**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_push_channel_id() {
        let t = WebPushTransport::new();
        assert_eq!(t.channel_id().as_str(), "push_web");
        assert_eq!(t.display_name(), "Push (Web)");
    }

    #[test]
    fn web_push_content_schema() {
        let t = WebPushTransport::new();
        let schema = t.content_schema();
        assert_eq!(schema.fields.len(), 6);
        let required: Vec<_> = schema.fields.iter().filter(|f| f.required).map(|f| f.name.as_str()).collect();
        assert!(required.contains(&"title"));
        assert!(required.contains(&"body"));
    }

    #[test]
    fn web_push_credential_schema() {
        let t = WebPushTransport::new();
        let schema = t.credential_schema();
        assert_eq!(schema.fields.len(), 3);
        let secret_fields: Vec<_> = schema.fields.iter().filter(|f| f.secret).map(|f| f.name.as_str()).collect();
        assert!(secret_fields.contains(&"vapid_private_key"));
    }

    #[test]
    fn web_push_invalid_subscription() {
        let t = WebPushTransport::new();
        let msg = RenderedMessage {
            channel: ChannelId::new("push_web"),
            recipient_contact: "not-valid-json".into(),
            content: serde_json::json!({"title": "Test", "body": "Hello"}),
            credentials: serde_json::json!({
                "vapid_private_key": "dGVzdA",
                "vapid_public_key": "dGVzdA",
                "subject": "mailto:test@example.com"
            }),
            attachments: vec![],
        };
        let result = tokio::runtime::Runtime::new().unwrap().block_on(t.send(&msg));
        assert!(result.is_err() || matches!(result, Ok(DeliveryResult::Failed { .. })));
    }
}
```

**Step 2: Implement WebPushTransport**

`transports/web-push/Cargo.toml`:
```toml
[package]
name = "notifico-transport-web-push"
version = "0.1.0"
edition = "2024"

[dependencies]
notifico-core = { path = "../../notifico-core" }
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
web-push = "0.10"
tracing.workspace = true

[dev-dependencies]
tokio.workspace = true
```

`transports/web-push/src/lib.rs`:
```rust
use async_trait::async_trait;
use notifico_core::error::CoreError;
use notifico_core::transport::{
    ChannelId, ContentField, ContentFieldType, ContentSchema,
    CredentialField, CredentialSchema, DeliveryResult, RenderedMessage, Transport,
};
use web_push::{
    ContentEncoding, SubscriptionInfo, VapidSignatureBuilder,
    WebPushClient, WebPushMessageBuilder,
};

pub struct WebPushTransport {
    client: WebPushClient,
}

impl WebPushTransport {
    pub fn new() -> Self {
        Self {
            client: WebPushClient::new().expect("Failed to create WebPushClient"),
        }
    }
}

impl Default for WebPushTransport {
    fn default() -> Self { Self::new() }
}

#[derive(serde::Deserialize)]
struct PushSubscription {
    endpoint: String,
    keys: PushKeys,
}

#[derive(serde::Deserialize)]
struct PushKeys {
    p256dh: String,
    auth: String,
}

#[async_trait]
impl Transport for WebPushTransport {
    fn channel_id(&self) -> ChannelId { ChannelId::new("push_web") }
    fn display_name(&self) -> &str { "Push (Web)" }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![
                ContentField { name: "title".into(), field_type: ContentFieldType::Text, required: true, description: "Notification title".into() },
                ContentField { name: "body".into(), field_type: ContentFieldType::Text, required: true, description: "Notification body".into() },
                ContentField { name: "icon".into(), field_type: ContentFieldType::Text, required: false, description: "Icon URL".into() },
                ContentField { name: "url".into(), field_type: ContentFieldType::Text, required: false, description: "Click destination URL".into() },
                ContentField { name: "badge".into(), field_type: ContentFieldType::Text, required: false, description: "Badge icon URL".into() },
                ContentField { name: "data".into(), field_type: ContentFieldType::Json, required: false, description: "Custom payload".into() },
            ],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema {
            fields: vec![
                CredentialField { name: "vapid_private_key".into(), required: true, secret: true, description: "VAPID private key (base64url)".into() },
                CredentialField { name: "vapid_public_key".into(), required: true, secret: false, description: "VAPID public key (base64url)".into() },
                CredentialField { name: "subject".into(), required: true, secret: false, description: "Contact URI (mailto: or https://)".into() },
            ],
        }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        let sub: PushSubscription = serde_json::from_str(&message.recipient_contact)
            .map_err(|e| CoreError::Transport(format!("Invalid push subscription: {e}")))?;

        let vapid_private = message.credentials.get("vapid_private_key").and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing vapid_private_key".into()))?;
        let subject = message.credentials.get("subject").and_then(|v| v.as_str())
            .ok_or_else(|| CoreError::Transport("Missing subject".into()))?;

        let subscription = SubscriptionInfo::new(sub.endpoint, sub.keys.p256dh, sub.keys.auth);

        // Build notification payload
        let mut payload = serde_json::json!({
            "title": message.content.get("title").and_then(|v| v.as_str()).unwrap_or(""),
            "body": message.content.get("body").and_then(|v| v.as_str()).unwrap_or(""),
        });
        if let Some(v) = message.content.get("icon").and_then(|v| v.as_str()) { payload["icon"] = v.into(); }
        if let Some(v) = message.content.get("url").and_then(|v| v.as_str()) { payload["url"] = v.into(); }
        if let Some(v) = message.content.get("badge").and_then(|v| v.as_str()) { payload["badge"] = v.into(); }
        if let Some(v) = message.content.get("data") { payload["data"] = v.clone(); }

        let payload_bytes = serde_json::to_vec(&payload)
            .map_err(|e| CoreError::Transport(format!("Payload serialize error: {e}")))?;

        let sig = VapidSignatureBuilder::from_base64_no_sub(vapid_private, ContentEncoding::Aes128Gcm)
            .map_err(|e| CoreError::Transport(format!("VAPID key error: {e}")))?
            .add_sub_info(&subscription)
            .add_claim("sub", subject)
            .build()
            .map_err(|e| CoreError::Transport(format!("VAPID sign error: {e}")))?;

        let mut builder = WebPushMessageBuilder::new(&subscription);
        builder.set_payload(ContentEncoding::Aes128Gcm, &payload_bytes);
        builder.set_vapid_signature(sig);

        let web_push_message = builder.build()
            .map_err(|e| CoreError::Transport(format!("WebPush build error: {e}")))?;

        match self.client.send(web_push_message).await {
            Ok(_) => Ok(DeliveryResult::Delivered { provider_message_id: None }),
            Err(e) => {
                let error_str = format!("{e}");
                let retryable = !error_str.contains("410") && !error_str.contains("Gone");
                Ok(DeliveryResult::Failed { error: error_str, retryable })
            }
        }
    }
}
```

**Note:** The exact `web-push` crate API may differ by version. Verify `VapidSignatureBuilder`, `SubscriptionInfo::new`, and `WebPushClient` signatures against the actual crate docs and adjust accordingly.

**Step 3: Register in server**

`notifico-server/Cargo.toml`: add `notifico-transport-web-push = { path = "../transports/web-push" }`

`main.rs`:
```rust
use notifico_transport_web_push::WebPushTransport;
// ...
registry.register(Arc::new(WebPushTransport::new()));
```

**Step 4: Verify**

Run: `cargo test -p notifico-transport-web-push`
Run: `cargo test -p notifico-server`

**Step 5: Commit**

```
feat: add Web Push transport (transports/web-push)
```

---

### Task 9: Final verification

**Step 1: Run full workspace tests**

Run: `cargo test --workspace`

All tests must pass across all crates.

**Step 2: Verify channel count**

The channels endpoint should now return 10 channels:
console, email, slack, discord, telegram, sms, webhook, push_fcm, push_apns, push_web

**Step 3: Build frontend (for embedded static files)**

Run: `cd notifico-frontend && bun run build`

**Step 4: Final commit if any cleanup needed**

**Step 5: Push**

```
git push origin main
```
