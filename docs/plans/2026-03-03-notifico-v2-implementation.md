# Notifico v2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Full rewrite of Notifico — a self-hosted notification server with multi-channel delivery, user preferences, template versioning, and mass broadcasts.

**Architecture:** Hybrid monolith (single binary, `--mode api/worker/all`). Rust backend (axum + sea-orm + apalis), React frontend (Shadcn + TanStack + GrapesJS). PostgreSQL/SQLite for storage, Valkey for cache/queue.

**Tech Stack:** Rust 2024, axum, sea-orm, apalis, minijinja, mrml, lettre, React 18, TypeScript, Shadcn/ui, TanStack Query/Router, Tailwind, GrapesJS, PostgreSQL 18, Valkey 9.

**Design doc:** `docs/plans/2026-03-03-notifico-v2-design.md`

---

## Phase Overview

| Phase | Name | Description |
|-------|------|-------------|
| 1 | Foundation | Workspace scaffolding, core types, DB models, config |
| 2 | Pipeline Engine | Template engine, queue, pipeline executor, delivery worker |
| 3 | API Layer | Ingest, Admin, Public APIs + auth |
| 4 | Transports | Email, SMS, Push, Messengers |
| 5 | Frontend | React admin panel + GrapesJS email editor |
| 6 | Advanced | Broadcasts, OIDC, observability, Docker |

Each phase is independently testable. Detailed task plans for phases 2-6 will be created when we reach them.

---

## Phase 1: Foundation

### Task 1: Clean repo and scaffold Cargo workspace

**Files:**
- Delete: all existing source files (keep `.git/`, `docs/`, `.idea/`)
- Create: `Cargo.toml` (workspace root)
- Create: `notifico-core/Cargo.toml`
- Create: `notifico-core/src/lib.rs`
- Create: `notifico-server/Cargo.toml`
- Create: `notifico-server/src/main.rs`
- Create: `notifico-db/Cargo.toml`
- Create: `notifico-db/src/lib.rs`
- Create: `rust-toolchain.toml`
- Create: `.gitignore`

**Step 1: Remove old source files**

Remove all old crates, Dockerfiles, container/ dir, etc. Keep `docs/` and `.git/`.

```bash
# Remove old source directories (keep docs/ and .git/)
rm -rf notifico-app/ notifico-core/ notifico-template/ notifico-attachment/
rm -rf notifico-transports/ transports/ notificox/ container/
rm -rf .cargo/ .github/ .dockerignore Dockerfile
rm -f Cargo.toml Cargo.lock README.md
```

**Step 2: Create workspace Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "notifico-core",
    "notifico-db",
    "notifico-server",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "Apache-2.0"
repository = "https://github.com/notificohq/notifico"

[workspace.dependencies]
# Core
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "2"
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Web
axum = { version = "0.8", features = ["macros"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Database
sea-orm = { version = "1.1", features = ["sqlx-postgres", "sqlx-sqlite", "runtime-tokio-rustls", "macros"] }
sea-orm-migration = { version = "1.1" }

# Config
toml = "0.8"
figment = { version = "0.10", features = ["toml", "env"] }

# Internal crates
notifico-core = { path = "notifico-core" }
notifico-db = { path = "notifico-db" }

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

**Step 3: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "stable"
```

**Step 4: Create .gitignore**

```
/target
*.swp
*.swo
.env
.DS_Store
```

**Step 5: Create notifico-core crate**

`notifico-core/Cargo.toml`:
```toml
[package]
name = "notifico-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
```

`notifico-core/src/lib.rs`:
```rust
pub mod channel;
pub mod error;
```

**Step 6: Create notifico-db crate**

`notifico-db/Cargo.toml`:
```toml
[package]
name = "notifico-db"
version.workspace = true
edition.workspace = true

[dependencies]
sea-orm = { workspace = true }
sea-orm-migration = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
notifico-core = { workspace = true }
```

`notifico-db/src/lib.rs`:
```rust
pub mod migration;
```

**Step 7: Create notifico-server crate**

`notifico-server/Cargo.toml`:
```toml
[package]
name = "notifico-server"
version.workspace = true
edition.workspace = true

[[bin]]
name = "notifico"
path = "src/main.rs"

[dependencies]
tokio = { workspace = true }
axum = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
notifico-core = { workspace = true }
notifico-db = { workspace = true }
```

`notifico-server/src/main.rs`:
```rust
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    tracing::info!("Notifico v2 starting...");
}
```

**Step 8: Verify workspace compiles**

Run: `cargo build`
Expected: compiles successfully

**Step 9: Commit**

```bash
git add -A
git commit -m "feat: scaffold Notifico v2 workspace

Clean repo and create Cargo workspace with three initial crates:
- notifico-core: core types and traits
- notifico-db: database models and migrations
- notifico-server: HTTP server binary"
```

---

### Task 2: Core types — ChannelId, Transport trait, errors

**Files:**
- Create: `notifico-core/src/channel.rs`
- Create: `notifico-core/src/transport.rs`
- Create: `notifico-core/src/error.rs`
- Create: `notifico-core/src/recipient.rs`
- Create: `notifico-core/src/event.rs`
- Modify: `notifico-core/src/lib.rs`

**Step 1: Write tests for ChannelId**

Create `notifico-core/src/channel.rs`:
```rust
use serde::{Deserialize, Serialize};

/// Channel identifier. String-based for extensibility (native + future WASM plugins).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelId(String);

impl ChannelId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_id_creation_and_display() {
        let ch = ChannelId::new("email");
        assert_eq!(ch.as_str(), "email");
        assert_eq!(ch.to_string(), "email");
    }

    #[test]
    fn channel_id_equality() {
        let a = ChannelId::new("sms");
        let b = ChannelId::new("sms");
        let c = ChannelId::new("email");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn channel_id_serialization() {
        let ch = ChannelId::new("telegram");
        let json = serde_json::to_string(&ch).unwrap();
        assert_eq!(json, "\"telegram\"");
        let deserialized: ChannelId = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ch);
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p notifico-core`
Expected: 3 tests pass

**Step 3: Write error types**

Create `notifico-core/src/error.rs`:
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Template rendering error: {0}")]
    TemplateRender(String),

    #[error("Recipient not found: {0}")]
    RecipientNotFound(String),

    #[error("Channel not registered: {0}")]
    ChannelNotRegistered(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
```

**Step 4: Write event types**

Create `notifico-core/src/event.rs`:
```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Notification category determines unsubscribe rules and delivery behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    /// Cannot unsubscribe (order confirmation, password reset)
    Transactional,
    /// Can unsubscribe, respects user schedules
    Marketing,
    /// Technical notifications, no user preferences
    System,
}

/// An ingest event sent by a client application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestEvent {
    /// Event name (e.g. "order.confirmed")
    pub event: String,
    /// Recipients to notify
    pub recipients: Vec<EventRecipient>,
    /// Template data
    pub data: serde_json::Value,
    /// Optional idempotency key
    pub idempotency_key: Option<String>,
}

/// Recipient within an ingest event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecipient {
    /// External ID from the client system
    pub id: String,
    /// Optional overrides for contact info
    #[serde(default)]
    pub contacts: std::collections::HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_category_serialization() {
        let cat = EventCategory::Transactional;
        let json = serde_json::to_string(&cat).unwrap();
        assert_eq!(json, "\"transactional\"");
    }

    #[test]
    fn ingest_event_deserialization() {
        let json = r#"{
            "event": "order.confirmed",
            "recipients": [{"id": "user-123", "contacts": {"email": "test@example.com"}}],
            "data": {"order_id": 42}
        }"#;
        let event: IngestEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event, "order.confirmed");
        assert_eq!(event.recipients.len(), 1);
        assert_eq!(event.recipients[0].id, "user-123");
    }
}
```

**Step 5: Write recipient types**

Create `notifico-core/src/recipient.rs`:
```rust
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::channel::ChannelId;

/// A recipient in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    pub id: Uuid,
    pub project_id: Uuid,
    pub external_id: String,
    pub locale: String,
    pub timezone: String,
    pub metadata: serde_json::Value,
}

/// A contact method for a recipient.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipientContact {
    pub id: Uuid,
    pub recipient_id: Uuid,
    pub channel: ChannelId,
    pub value: String,
    pub verified: bool,
}
```

**Step 6: Write Transport trait**

Create `notifico-core/src/transport.rs`:
```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::channel::ChannelId;
use crate::error::CoreError;

/// Schema describing what fields a channel needs in template content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentSchema {
    pub fields: Vec<ContentField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentField {
    pub name: String,
    pub field_type: ContentFieldType,
    pub required: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentFieldType {
    Text,
    Html,
    Json,
}

/// Schema describing what credentials a transport needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSchema {
    pub fields: Vec<CredentialField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialField {
    pub name: String,
    pub required: bool,
    pub secret: bool,
    pub description: String,
}

/// A fully rendered message ready for delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedMessage {
    pub channel: ChannelId,
    pub recipient_contact: String,
    pub content: Value,
    pub credentials: Value,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub disposition: AttachmentDisposition,
    pub content_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentDisposition {
    Inline,
    Attachment,
}

/// Result of a delivery attempt.
#[derive(Debug, Clone)]
pub enum DeliveryResult {
    Delivered { provider_message_id: Option<String> },
    Failed { error: String, retryable: bool },
}

/// The core transport trait. All channels implement this.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Unique channel identifier.
    fn channel_id(&self) -> ChannelId;

    /// Human-readable name for admin UI.
    fn display_name(&self) -> &str;

    /// Schema for template content fields.
    fn content_schema(&self) -> ContentSchema;

    /// Schema for required credentials.
    fn credential_schema(&self) -> CredentialSchema;

    /// Send a rendered message.
    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError>;
}
```

**Step 7: Update lib.rs**

`notifico-core/src/lib.rs`:
```rust
pub mod channel;
pub mod error;
pub mod event;
pub mod recipient;
pub mod transport;
```

**Step 8: Run all tests**

Run: `cargo test -p notifico-core`
Expected: all tests pass

**Step 9: Commit**

```bash
git add -A
git commit -m "feat(core): add core types — ChannelId, Transport trait, Event, Recipient

- ChannelId: string-based for extensibility (native + future WASM)
- Transport trait: async send with content/credential schemas
- IngestEvent: event model for API ingestion
- EventCategory: transactional/marketing/system
- Recipient + RecipientContact types
- CoreError error types"
```

---

### Task 3: Transport registry

**Files:**
- Create: `notifico-core/src/registry.rs`
- Modify: `notifico-core/src/lib.rs`

**Step 1: Write tests for TransportRegistry**

Create `notifico-core/src/registry.rs`:
```rust
use std::collections::HashMap;
use std::sync::Arc;

use crate::channel::ChannelId;
use crate::transport::{ContentSchema, CredentialSchema, Transport};

/// Registry of available transports. Populated at startup.
pub struct TransportRegistry {
    transports: HashMap<ChannelId, Arc<dyn Transport>>,
}

impl TransportRegistry {
    pub fn new() -> Self {
        Self {
            transports: HashMap::new(),
        }
    }

    /// Register a transport. Panics if channel_id is already registered.
    pub fn register(&mut self, transport: Arc<dyn Transport>) {
        let channel_id = transport.channel_id();
        if self.transports.contains_key(&channel_id) {
            panic!("Transport already registered for channel: {}", channel_id);
        }
        self.transports.insert(channel_id, transport);
    }

    /// Get a transport by channel ID.
    pub fn get(&self, channel_id: &ChannelId) -> Option<&Arc<dyn Transport>> {
        self.transports.get(channel_id)
    }

    /// List all registered channel IDs.
    pub fn channels(&self) -> Vec<ChannelId> {
        self.transports.keys().cloned().collect()
    }

    /// Get channel info for admin UI.
    pub fn channel_info(&self) -> Vec<ChannelInfo> {
        self.transports
            .values()
            .map(|t| ChannelInfo {
                channel_id: t.channel_id(),
                display_name: t.display_name().to_string(),
                content_schema: t.content_schema(),
                credential_schema: t.credential_schema(),
            })
            .collect()
    }
}

impl Default for TransportRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub channel_id: ChannelId,
    pub display_name: String,
    pub content_schema: ContentSchema,
    pub credential_schema: CredentialSchema,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CoreError;
    use crate::transport::*;
    use async_trait::async_trait;

    struct FakeTransport {
        id: ChannelId,
    }

    #[async_trait]
    impl Transport for FakeTransport {
        fn channel_id(&self) -> ChannelId {
            self.id.clone()
        }
        fn display_name(&self) -> &str {
            "Fake"
        }
        fn content_schema(&self) -> ContentSchema {
            ContentSchema { fields: vec![] }
        }
        fn credential_schema(&self) -> CredentialSchema {
            CredentialSchema { fields: vec![] }
        }
        async fn send(&self, _msg: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
            Ok(DeliveryResult::Delivered {
                provider_message_id: None,
            })
        }
    }

    #[test]
    fn register_and_get_transport() {
        let mut registry = TransportRegistry::new();
        let transport = Arc::new(FakeTransport {
            id: ChannelId::new("fake"),
        });
        registry.register(transport);

        assert!(registry.get(&ChannelId::new("fake")).is_some());
        assert!(registry.get(&ChannelId::new("missing")).is_none());
    }

    #[test]
    fn list_channels() {
        let mut registry = TransportRegistry::new();
        registry.register(Arc::new(FakeTransport {
            id: ChannelId::new("email"),
        }));
        registry.register(Arc::new(FakeTransport {
            id: ChannelId::new("sms"),
        }));

        let channels = registry.channels();
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&ChannelId::new("email")));
        assert!(channels.contains(&ChannelId::new("sms")));
    }

    #[test]
    #[should_panic(expected = "Transport already registered")]
    fn duplicate_registration_panics() {
        let mut registry = TransportRegistry::new();
        registry.register(Arc::new(FakeTransport {
            id: ChannelId::new("email"),
        }));
        registry.register(Arc::new(FakeTransport {
            id: ChannelId::new("email"),
        }));
    }
}
```

**Step 2: Update lib.rs**

Add `pub mod registry;` to `notifico-core/src/lib.rs`.

**Step 3: Run tests**

Run: `cargo test -p notifico-core`
Expected: all tests pass (previous + 3 new)

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(core): add TransportRegistry

Registry for dynamic transport discovery. Supports register,
lookup by ChannelId, list channels, and channel info for admin UI."
```

---

### Task 4: Configuration system

**Files:**
- Create: `notifico-server/src/config.rs`
- Modify: `notifico-server/src/main.rs`
- Modify: `notifico-server/Cargo.toml`

**Step 1: Write config types**

Create `notifico-server/src/config.rs`:
```rust
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub queue: QueueConfig,
    #[serde(default)]
    pub storage: StorageConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub project: ProjectConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_mode")]
    pub mode: ServerMode,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_admin_port")]
    pub admin_port: u16,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServerMode {
    All,
    Api,
    Worker,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_backend")]
    pub backend: String,
    #[serde(default = "default_db_url")]
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    #[serde(default = "default_queue_backend")]
    pub backend: String,
    #[serde(default)]
    pub redis_url: Option<String>,
    #[serde(default)]
    pub amqp_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_storage_backend")]
    pub backend: String,
    #[serde(default = "default_storage_path")]
    pub path: String,
    #[serde(default)]
    pub s3_bucket: Option<String>,
    #[serde(default)]
    pub s3_endpoint: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub encryption_key: Option<String>,
    #[serde(default)]
    pub jwt_secret: Option<String>,
    #[serde(default)]
    pub oidc: Option<OidcConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OidcConfig {
    pub enabled: bool,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectConfig {
    #[serde(default = "default_locale")]
    pub default_locale: String,
}

fn default_mode() -> ServerMode { ServerMode::All }
fn default_host() -> String { "0.0.0.0".into() }
fn default_port() -> u16 { 8000 }
fn default_admin_port() -> u16 { 8001 }
fn default_db_backend() -> String { "sqlite".into() }
fn default_db_url() -> String { "sqlite://notifico.db?mode=rwc".into() }
fn default_queue_backend() -> String { "redis".into() }
fn default_storage_backend() -> String { "filesystem".into() }
fn default_storage_path() -> String { "./data/assets".into() }
fn default_locale() -> String { "en".into() }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            host: default_host(),
            port: default_port(),
            admin_port: default_admin_port(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            backend: default_db_backend(),
            url: default_db_url(),
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            backend: default_queue_backend(),
            redis_url: None,
            amqp_url: None,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: default_storage_backend(),
            path: default_storage_path(),
            s3_bucket: None,
            s3_endpoint: None,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            encryption_key: None,
            jwt_secret: None,
            oidc: None,
        }
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            default_locale: default_locale(),
        }
    }
}

impl Config {
    /// Load config from notifico.toml (optional) + NOTIFICO_ env vars.
    pub fn load(config_path: Option<&str>) -> Result<Self, figment::Error> {
        let mut figment = Figment::new();

        if let Some(path) = config_path {
            figment = figment.merge(Toml::file(path));
        } else {
            // Try default path, don't fail if missing
            figment = figment.merge(Toml::file("notifico.toml").nested());
        }

        figment = figment.merge(Env::prefixed("NOTIFICO_").split("_"));

        figment.extract()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_loads() {
        let config = Config::load(None).unwrap();
        assert_eq!(config.server.mode, ServerMode::All);
        assert_eq!(config.server.port, 8000);
        assert_eq!(config.database.backend, "sqlite");
        assert_eq!(config.project.default_locale, "en");
    }
}
```

**Step 2: Add figment and toml deps to notifico-server**

Add to `notifico-server/Cargo.toml` dependencies:
```toml
figment = { workspace = true }
toml = { workspace = true }
```

**Step 3: Update main.rs to load config**

```rust
mod config;

use config::Config;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".parse().unwrap()),
        )
        .init();

    let config = Config::load(None).expect("Failed to load configuration");

    tracing::info!(
        mode = ?config.server.mode,
        port = config.server.port,
        db = config.database.backend.as_str(),
        "Notifico v2 starting"
    );
}
```

**Step 4: Run tests and build**

Run: `cargo test && cargo build`
Expected: all tests pass, builds successfully

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(server): add configuration system

Figment-based config: notifico.toml + NOTIFICO_ env overrides.
Server mode (all/api/worker), database, queue, storage, auth, project settings."
```

---

### Task 5: Database models and migrations (sea-orm)

**Files:**
- Create: `notifico-db/src/migration/mod.rs`
- Create: `notifico-db/src/migration/m20260303_000001_create_projects.rs`
- Create: `notifico-db/src/migration/m20260303_000002_create_events.rs`
- Create: `notifico-db/src/migration/m20260303_000003_create_templates.rs`
- Create: `notifico-db/src/migration/m20260303_000004_create_recipients.rs`
- Create: `notifico-db/src/migration/m20260303_000005_create_delivery_log.rs`
- Create: `notifico-db/src/migration/m20260303_000006_create_api_keys.rs`
- Create: `notifico-db/src/entities/` (one file per entity)
- Modify: `notifico-db/src/lib.rs`

**Step 1: Write migration module**

Create `notifico-db/src/migration/mod.rs`:
```rust
use sea_orm_migration::prelude::*;

mod m20260303_000001_create_projects;
mod m20260303_000002_create_events;
mod m20260303_000003_create_templates;
mod m20260303_000004_create_recipients;
mod m20260303_000005_create_delivery_log;
mod m20260303_000006_create_api_keys;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260303_000001_create_projects::Migration),
            Box::new(m20260303_000002_create_events::Migration),
            Box::new(m20260303_000003_create_templates::Migration),
            Box::new(m20260303_000004_create_recipients::Migration),
            Box::new(m20260303_000005_create_delivery_log::Migration),
            Box::new(m20260303_000006_create_api_keys::Migration),
        ]
    }
}
```

**Step 2: Write migrations**

Each migration file creates the tables from the data model in the design doc. These are standard sea-orm migrations using `SchemaManager`. Use `ColumnDef::new(...)` with appropriate types. Use `ColumnType::JsonBinary` for JSONB fields (falls back to TEXT on SQLite).

Full migration code for each file should follow the sea-orm-migration pattern. Key tables:
- `project` (id, name, default_locale, settings, created_at, updated_at)
- `event` (id, project_id FK, name, category, description)
- `pipeline_rule` (id, event_id FK, channel, template_id FK, enabled, conditions, priority)
- `template`, `template_version`, `template_content` (with locale)
- `recipient`, `recipient_contact`, `recipient_preference`
- `unsubscribe` (with token)
- `delivery_log`
- `api_key` (id, project_id FK, key_hash, prefix, scopes, rate_limit, created_at)

**Step 3: Write sea-orm entity files**

Generate entity structs using `DeriveEntityModel` for each table. Place in `notifico-db/src/entities/`.

**Step 4: Write DB connection helper**

Create `notifico-db/src/lib.rs` with:
- `pub mod migration;`
- `pub mod entities;`
- `pub async fn connect(url: &str) -> Result<DatabaseConnection>` helper
- `pub async fn run_migrations(db: &DatabaseConnection) -> Result<()>` helper

**Step 5: Test migrations run against SQLite**

Run: `cargo test -p notifico-db`
Write an integration test that connects to in-memory SQLite and runs all migrations.

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(db): add sea-orm migrations and entity models

6 migrations creating all core tables: projects, events, pipeline_rules,
templates (with versioning + locale content), recipients (with contacts
and preferences), unsubscribe, delivery_log, api_keys."
```

---

### Task 6: Wire up DB in server, run migrations on startup

**Files:**
- Modify: `notifico-server/src/main.rs`
- Modify: `notifico-server/Cargo.toml`

**Step 1: Add sea-orm to server deps**

**Step 2: Connect to DB and run migrations in main.rs**

```rust
// In main(), after config load:
let db = notifico_db::connect(&config.database.url)
    .await
    .expect("Failed to connect to database");

notifico_db::run_migrations(&db)
    .await
    .expect("Failed to run migrations");

tracing::info!("Database connected and migrations complete");
```

**Step 3: Verify**

Run: `cargo run` — should start, connect to SQLite, run migrations, and print log.

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(server): wire up database connection and auto-migrations"
```

---

### Task 7: Basic axum HTTP server with health endpoint

**Files:**
- Create: `notifico-server/src/routes/mod.rs`
- Create: `notifico-server/src/routes/health.rs`
- Modify: `notifico-server/src/main.rs`

**Step 1: Write health endpoint**

Create `notifico-server/src/routes/health.rs`:
```rust
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({"status": "ok"})))
}

pub async fn ready() -> impl IntoResponse {
    // TODO: check DB and queue connectivity
    (StatusCode::OK, Json(json!({"status": "ready"})))
}
```

Create `notifico-server/src/routes/mod.rs`:
```rust
pub mod health;
```

**Step 2: Wire up axum router in main.rs**

```rust
use axum::Router;

// Build router
let app = Router::new()
    .route("/health", axum::routing::get(routes::health::health))
    .route("/ready", axum::routing::get(routes::health::ready));

// Start server
let addr = format!("{}:{}", config.server.host, config.server.port);
let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
tracing::info!("Listening on {}", addr);
axum::serve(listener, app).await.unwrap();
```

**Step 3: Test manually**

Run: `cargo run &` then `curl http://localhost:8000/health`
Expected: `{"status":"ok"}`

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(server): add axum HTTP server with /health and /ready endpoints"
```

---

## Phase 2: Pipeline Engine (high-level)

Detailed task plan created when Phase 1 is complete.

- **Task 8:** Template engine — minijinja integration, render with context
- **Task 9:** Template versioning and locale resolution
- **Task 10:** Queue abstraction — apalis setup with configurable backend
- **Task 11:** Delivery task types (serializable job structs)
- **Task 12:** Pipeline executor — event → rules → render → enqueue
- **Task 13:** Delivery worker — dequeue → Transport::send → log result
- **Task 14:** Retry logic with exponential backoff
- **Task 15:** Idempotency and deduplication

## Phase 3: API Layer (high-level)

- **Task 16:** Ingest API — `POST /api/v1/events`
- **Task 17:** API key authentication middleware
- **Task 18:** Admin API — project CRUD
- **Task 19:** Admin API — event + pipeline rule CRUD
- **Task 20:** Admin API — template CRUD (versions, locales, content)
- **Task 21:** Admin API — recipient CRUD (contacts, preferences)
- **Task 22:** Admin API — credentials CRUD (encrypted storage)
- **Task 23:** Admin API — channels list + schema
- **Task 24:** Admin API — delivery log query
- **Task 25:** Admin API — API key management
- **Task 26:** Public API — recipient preferences
- **Task 27:** Public API — unsubscribe (List-Unsubscribe RFC 8058)
- **Task 28:** JWT auth for admin panel (login, refresh, CSRF)
- **Task 29:** OpenAPI spec generation (utoipa)

## Phase 4: Transports (high-level)

- **Task 30:** notifico-smtp (lettre + mrml + html2text)
- **Task 31:** notifico-sms-twilio
- **Task 32:** notifico-push-fcm
- **Task 33:** notifico-push-apns
- **Task 34:** notifico-push-web (Web Push RFC 8030 + VAPID)
- **Task 35:** notifico-telegram
- **Task 36:** notifico-max (VK MAX Bot API)
- **Task 37:** notifico-discord
- **Task 38:** notifico-slack

## Phase 5: Frontend (high-level)

- **Task 39:** React app scaffolding (Vite, Shadcn, TanStack, Tailwind)
- **Task 40:** OpenAPI → TypeScript client generation
- **Task 41:** Auth flow (login page, JWT, token refresh)
- **Task 42:** Dashboard page
- **Task 43:** Events + pipeline rules pages
- **Task 44:** Template list + editor pages
- **Task 45:** GrapesJS email block editor integration
- **Task 46:** Recipients + contacts + preferences pages
- **Task 47:** Broadcasts page
- **Task 48:** Delivery log page
- **Task 49:** Channels + credentials pages
- **Task 50:** API keys page
- **Task 51:** Settings page
- **Task 52:** Embed frontend in Rust binary (rust-embed)

## Phase 6: Advanced (high-level)

- **Task 53:** Broadcast engine (batch processing, rate limiting, scheduling)
- **Task 54:** OIDC authentication
- **Task 55:** Prometheus /metrics endpoint
- **Task 56:** OpenTelemetry integration (metrics + traces)
- **Task 57:** Dockerfile + docker-compose (simple + HA)
- **Task 58:** CLI companion (notificox) basic commands
- **Task 59:** README and getting started guide