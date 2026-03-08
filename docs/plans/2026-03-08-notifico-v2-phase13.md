# Phase 13: Pipeline Middleware, OpenTelemetry & Tracking

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a middleware system to the notification pipeline with day-1 middleware (unsubscribe link, click/open tracking, UTM params, plaintext fallback), OpenTelemetry tracing, and tracking endpoints.

**Architecture:** Middleware trait with 4 hook points (pre/post-render, pre/post-send). Middleware activation stored per pipeline rule in DB. Native Rust implementations. OpenTelemetry spans across pipeline + transports.

**Tech Stack:** Rust, async-trait, opentelemetry + tracing-opentelemetry, html2text, sea-orm migrations

**Design doc:** `docs/plans/2026-03-08-notifico-v2-frontend-design.md` (middleware section)

---

## Task 51: Middleware trait and registry

Add the core `Middleware` trait to notifico-core with 4 hook points and a `MiddlewareRegistry` for looking up middleware by name.

**Files:**
- Create: `notifico-core/src/middleware.rs`
- Modify: `notifico-core/src/lib.rs`

**Step 1: Create middleware trait and registry**

`notifico-core/src/middleware.rs`:
```rust
use async_trait::async_trait;
use serde_json::Value;

use crate::error::CoreError;
use crate::pipeline::{PipelineInput, PipelineOutput};
use crate::transport::{DeliveryResult, RenderedMessage};

/// Hook point in the pipeline where middleware runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookPoint {
    PreRender,
    PostRender,
    PreSend,
    PostSend,
}

/// Pipeline middleware trait. All hooks have default no-op implementations.
/// `config` is per-rule middleware configuration from the database.
#[async_trait]
pub trait Middleware: Send + Sync {
    fn name(&self) -> &str;

    async fn pre_render(
        &self,
        _input: &mut PipelineInput,
        _config: &Value,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    async fn post_render(
        &self,
        _output: &mut PipelineOutput,
        _config: &Value,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    async fn pre_send(
        &self,
        _message: &mut RenderedMessage,
        _config: &Value,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    async fn post_send(
        &self,
        _message: &RenderedMessage,
        _result: &DeliveryResult,
        _config: &Value,
    ) -> Result<(), CoreError> {
        Ok(())
    }
}

/// Registry of available middleware implementations.
pub struct MiddlewareRegistry {
    middleware: std::collections::HashMap<String, std::sync::Arc<dyn Middleware>>,
}

impl MiddlewareRegistry {
    pub fn new() -> Self {
        Self {
            middleware: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, mw: std::sync::Arc<dyn Middleware>) {
        let name = mw.name().to_string();
        self.middleware.insert(name, mw);
    }

    pub fn get(&self, name: &str) -> Option<&std::sync::Arc<dyn Middleware>> {
        self.middleware.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        self.middleware.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for MiddlewareRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct NoopMiddleware;

    #[async_trait]
    impl Middleware for NoopMiddleware {
        fn name(&self) -> &str {
            "noop"
        }
    }

    #[test]
    fn register_and_lookup() {
        let mut registry = MiddlewareRegistry::new();
        registry.register(Arc::new(NoopMiddleware));
        assert!(registry.get("noop").is_some());
        assert!(registry.get("missing").is_none());
    }

    #[test]
    fn list_middleware() {
        let mut registry = MiddlewareRegistry::new();
        registry.register(Arc::new(NoopMiddleware));
        let names = registry.list();
        assert!(names.contains(&"noop"));
    }

    #[tokio::test]
    async fn default_hooks_are_noop() {
        let mw = NoopMiddleware;
        let config = serde_json::json!({});
        let mut input = crate::pipeline::PipelineInput {
            project_id: uuid::Uuid::now_v7(),
            event_name: "test".into(),
            recipient_id: uuid::Uuid::now_v7(),
            recipient_locale: "en".into(),
            channel: "email".into(),
            contact_value: "test@test.com".into(),
            template_body: serde_json::json!({"text": "hi"}),
            context_data: serde_json::json!({}),
            idempotency_key: None,
            max_attempts: 3,
        };
        assert!(mw.pre_render(&mut input, &config).await.is_ok());
    }
}
```

**Step 2: Update lib.rs**

Add `pub mod middleware;` to `notifico-core/src/lib.rs`.

**Step 3: Run tests**

Run: `cargo test -p notifico-core`
Expected: all existing tests + 3 new middleware tests pass.

**Step 4: Commit**

```
feat(core): add Middleware trait and MiddlewareRegistry
```

---

## Task 52: DB migration for pipeline_middleware table

**Files:**
- Create: `notifico-db/src/migration/m20260308_000010_create_pipeline_middleware.rs`
- Modify: `notifico-db/src/migration/mod.rs`

**Step 1: Write migration**

`notifico-db/src/migration/m20260308_000010_create_pipeline_middleware.rs`:
```rust
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260308_000010_create_pipeline_middleware"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE pipeline_middleware (
                    id TEXT PRIMARY KEY,
                    rule_id TEXT NOT NULL REFERENCES pipeline_rule(id) ON DELETE CASCADE,
                    middleware_name TEXT NOT NULL,
                    config TEXT NOT NULL DEFAULT '{}',
                    priority INTEGER NOT NULL DEFAULT 0,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                )"
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                "CREATE INDEX idx_pipeline_middleware_rule_id ON pipeline_middleware(rule_id, priority)"
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS pipeline_middleware")
            .await?;
        Ok(())
    }
}
```

**Step 2: Register migration in mod.rs**

Add `mod m20260308_000010_create_pipeline_middleware;` and add to the migrations vec.

**Step 3: Run tests**

Run: `cargo test -p notifico-db`
Expected: migration tests pass (idempotent, runs on SQLite).

**Step 4: Commit**

```
feat(db): add pipeline_middleware table migration
```

---

## Task 53: Middleware repo functions

**Files:**
- Create: `notifico-db/src/repo/middleware.rs`
- Modify: `notifico-db/src/repo/mod.rs`

**Step 1: Write repo functions with tests**

`notifico-db/src/repo/middleware.rs`:
```rust
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, FromQueryResult)]
pub struct MiddlewareRow {
    pub id: String,
    pub rule_id: String,
    pub middleware_name: String,
    pub config: String,
    pub priority: i32,
    pub enabled: bool,
}

impl MiddlewareRow {
    pub fn config_value(&self) -> Value {
        serde_json::from_str(&self.config).unwrap_or(Value::Object(Default::default()))
    }
}

/// List middleware for a pipeline rule, ordered by priority (ascending).
pub async fn list_by_rule(
    db: &DatabaseConnection,
    rule_id: Uuid,
) -> Result<Vec<MiddlewareRow>, DbErr> {
    MiddlewareRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, rule_id, middleware_name, config, priority, enabled FROM pipeline_middleware WHERE rule_id = ? ORDER BY priority ASC",
        [rule_id.to_string().into()],
    ))
    .all(db)
    .await
}

/// Insert a middleware entry for a pipeline rule.
pub async fn insert(
    db: &DatabaseConnection,
    id: Uuid,
    rule_id: Uuid,
    middleware_name: &str,
    config: &Value,
    priority: i32,
) -> Result<(), DbErr> {
    let config_str = serde_json::to_string(config).unwrap_or_else(|_| "{}".into());
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO pipeline_middleware (id, rule_id, middleware_name, config, priority, enabled) VALUES (?, ?, ?, ?, ?, 1)",
        [
            id.to_string().into(),
            rule_id.to_string().into(),
            middleware_name.into(),
            config_str.into(),
            priority.into(),
        ],
    ))
    .await?;
    Ok(())
}

/// Update middleware config, priority, or enabled status.
pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    config: &Value,
    priority: i32,
    enabled: bool,
) -> Result<(), DbErr> {
    let config_str = serde_json::to_string(config).unwrap_or_else(|_| "{}".into());
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE pipeline_middleware SET config = ?, priority = ?, enabled = ? WHERE id = ?",
        [
            config_str.into(),
            priority.into(),
            enabled.into(),
            id.to_string().into(),
        ],
    ))
    .await?;
    Ok(())
}

/// Delete a middleware entry.
pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM pipeline_middleware WHERE id = ?",
        [id.to_string().into()],
    ))
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> (DatabaseConnection, Uuid) {
        let db = crate::connect("sqlite::memory:").await.unwrap();
        crate::run_migrations(&db).await.unwrap();

        let project_id = Uuid::now_v7();
        let event_id = Uuid::now_v7();
        let template_id = Uuid::now_v7();
        let rule_id = Uuid::now_v7();

        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();
        db.execute_unprepared(&format!(
            "INSERT INTO event (id, project_id, name, category) VALUES ('{event_id}', '{project_id}', 'test.event', 'transactional')"
        ))
        .await
        .unwrap();
        db.execute_unprepared(&format!(
            "INSERT INTO template (id, project_id, name, channel) VALUES ('{template_id}', '{project_id}', 'tpl', 'email')"
        ))
        .await
        .unwrap();
        db.execute_unprepared(&format!(
            "INSERT INTO pipeline_rule (id, event_id, channel, template_id, enabled, priority) VALUES ('{rule_id}', '{event_id}', 'email', '{template_id}', true, 10)"
        ))
        .await
        .unwrap();

        (db, rule_id)
    }

    #[tokio::test]
    async fn crud_middleware() {
        let (db, rule_id) = setup_db().await;

        // Insert
        let mw_id = Uuid::now_v7();
        insert(&db, mw_id, rule_id, "unsubscribe_link", &serde_json::json!({"url": "/unsub"}), 10)
            .await
            .unwrap();

        // List
        let items = list_by_rule(&db, rule_id).await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].middleware_name, "unsubscribe_link");
        assert_eq!(items[0].priority, 10);

        // Update
        update(&db, mw_id, &serde_json::json!({"url": "/unsub2"}), 5, false)
            .await
            .unwrap();
        let items = list_by_rule(&db, rule_id).await.unwrap();
        assert_eq!(items[0].priority, 5);

        // Delete
        delete(&db, mw_id).await.unwrap();
        let items = list_by_rule(&db, rule_id).await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn middleware_ordered_by_priority() {
        let (db, rule_id) = setup_db().await;

        insert(&db, Uuid::now_v7(), rule_id, "b_second", &serde_json::json!({}), 20).await.unwrap();
        insert(&db, Uuid::now_v7(), rule_id, "a_first", &serde_json::json!({}), 10).await.unwrap();

        let items = list_by_rule(&db, rule_id).await.unwrap();
        assert_eq!(items[0].middleware_name, "a_first");
        assert_eq!(items[1].middleware_name, "b_second");
    }
}
```

**Step 2: Add `pub mod middleware;` to `notifico-db/src/repo/mod.rs`**

**Step 3: Run tests**

Run: `cargo test -p notifico-db`
Expected: all tests pass.

**Step 4: Commit**

```
feat(db): add middleware repo functions
```

---

## Task 54: Integrate middleware into pipeline and worker

Wire middleware execution into the ingest pipeline (post-render) and delivery worker (pre-send, post-send). Add `MiddlewareRegistry` to `AppState`.

**Files:**
- Modify: `notifico-server/src/main.rs` — add MiddlewareRegistry to AppState
- Modify: `notifico-server/src/ingest.rs` — run post-render middleware after execute_pipeline
- Modify: `notifico-server/src/worker.rs` — run pre-send/post-send middleware around transport.send()

**Step 1: Add MiddlewareRegistry to AppState in main.rs**

Add `use notifico_core::middleware::MiddlewareRegistry;` and add field `pub(crate) middleware_registry: MiddlewareRegistry` to `AppState`. Initialize empty in main(). Update `setup_app` and `setup_admin_app` test helpers to include the field.

**Step 2: Run middleware in ingest.rs**

After `execute_pipeline(input)` produces a `PipelineOutput`, fetch middleware for the rule via `repo::middleware::list_by_rule`, then for each enabled middleware entry, call `middleware_registry.get(name)?.post_render(&mut output, &config)`.

**Step 3: Run middleware in worker.rs**

In `process_delivery()`, before `transport.send()` call pre-send middleware. After `transport.send()` call post-send middleware. Fetch middleware config from DB using the delivery task's rule context.

Note: The delivery task currently doesn't store rule_id. Add an optional `rule_id` field to `DeliveryTask` in notifico-queue and propagate from ingest. For tasks without rule_id, skip middleware.

**Step 4: Run all tests**

Run: `cargo test`
Expected: all tests pass (existing behavior unchanged — no middleware registered yet).

**Step 5: Commit**

```
feat: integrate middleware execution into pipeline and worker
```

---

## Task 55: Admin API for middleware CRUD

**Files:**
- Modify: `notifico-server/src/admin.rs`

**Step 1: Add middleware endpoints to admin router**

Add to admin_router():
```rust
.route("/rules/{rule_id}/middleware", get(list_middleware).post(create_middleware))
.route("/middleware/{id}", put(update_middleware).delete(delete_middleware))
```

**Step 2: Implement handlers**

- `list_middleware` — calls `repo::middleware::list_by_rule`, returns JSON array
- `create_middleware` — accepts `{"middleware_name", "config", "priority"}`, inserts
- `update_middleware` — accepts `{"config", "priority", "enabled"}`, updates
- `delete_middleware` — deletes by id

**Step 3: Add integration test**

In `notifico-server/src/main.rs` integration tests:
```rust
#[tokio::test]
async fn admin_middleware_crud() {
    // Create event + template + rule
    // POST middleware to rule
    // GET middleware list
    // PUT update
    // DELETE
}
```

**Step 4: Run tests**

Run: `cargo test`
Expected: all tests pass including new middleware CRUD test.

**Step 5: Commit**

```
feat: add admin API endpoints for middleware CRUD
```

---

## Task 56: OpenTelemetry tracing setup

Add OpenTelemetry with OTLP exporter. Instrument pipeline execution, middleware chain, and transport sends with spans.

**Files:**
- Modify: `Cargo.toml` — add opentelemetry workspace deps
- Modify: `notifico-server/Cargo.toml` — add opentelemetry deps
- Modify: `notifico-server/src/main.rs` — initialize OTel subscriber
- Modify: `notifico-server/src/config.rs` — add otel config

**Step 1: Add workspace dependencies**

In root `Cargo.toml` [workspace.dependencies]:
```toml
opentelemetry = "0.29"
opentelemetry_sdk = { version = "0.29", features = ["rt-tokio"] }
opentelemetry-otlp = { version = "0.29", features = ["grpc-tonic"] }
tracing-opentelemetry = "0.30"
```

**Step 2: Add OTel config**

In `notifico-server/src/config.rs`, add to Config:
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct OtelConfig {
    #[serde(default)]
    pub endpoint: Option<String>,  // OTLP endpoint, e.g. "http://localhost:4317"
    #[serde(default = "default_service_name")]
    pub service_name: String,
}
```

**Step 3: Initialize OTel in main.rs**

If `config.otel.endpoint` is set, create OTLP exporter and layer it into tracing subscriber. Otherwise, use tracing-only (no export).

**Step 4: Instrument pipeline and worker**

Add `#[tracing::instrument]` spans to:
- `execute_pipeline` — span with event_name, channel
- Each middleware call — span with middleware name, hook point
- `transport.send()` — span with channel, recipient (redacted)

**Step 5: Run tests**

Run: `cargo test`
Expected: all tests pass (OTel disabled by default in tests).

**Step 6: Commit**

```
feat: add OpenTelemetry tracing with OTLP export
```

---

## Task 57: Tracking endpoints and DB table

Add tracking_event table and endpoints for open/click tracking.

**Files:**
- Create: `notifico-db/src/migration/m20260308_000011_create_tracking_event.rs`
- Create: `notifico-db/src/repo/tracking.rs`
- Create: `notifico-server/src/tracking.rs`
- Modify: `notifico-db/src/migration/mod.rs`
- Modify: `notifico-db/src/repo/mod.rs`
- Modify: `notifico-server/src/main.rs`

**Step 1: Migration**

```sql
CREATE TABLE tracking_event (
    id TEXT PRIMARY KEY,
    delivery_log_id TEXT,
    event_type TEXT NOT NULL,  -- 'open' or 'click'
    url TEXT,                  -- original URL for clicks
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_tracking_event_delivery ON tracking_event(delivery_log_id);
```

**Step 2: Repo functions**

- `insert_tracking_event(db, id, delivery_log_id, event_type, url)`
- `count_by_delivery(db, delivery_log_id)` — for stats

**Step 3: Tracking endpoints**

- `GET /t/open/{token}` — decode token (base64 of delivery_log_id), record open event, return 1x1 transparent GIF
- `GET /t/click/{token}` — decode token (base64 of delivery_log_id + url), record click event, 302 redirect to original URL

Token format: base64url-encoded JSON `{"d": "delivery_log_id", "u": "https://..."}` signed with HMAC using the encryption_key.

**Step 4: Add routes to main.rs**

```rust
.route("/t/open/{token}", get(tracking::handle_open))
.route("/t/click/{token}", get(tracking::handle_click))
```

**Step 5: Integration test**

Test that `/t/open/{token}` returns a GIF and `/t/click/{token}` returns 302.

**Step 6: Commit**

```
feat: add tracking endpoints for open/click analytics
```

---

## Task 58: Unsubscribe link middleware

**Files:**
- Create: `notifico-core/src/middleware/unsubscribe_link.rs`
- Modify: `notifico-core/src/middleware.rs` → convert to `notifico-core/src/middleware/mod.rs`

**Step 1: Convert middleware.rs to module directory**

Move `notifico-core/src/middleware.rs` to `notifico-core/src/middleware/mod.rs`, add `pub mod unsubscribe_link;`.

**Step 2: Implement UnsubscribeLinkMiddleware**

Post-render middleware that:
- Adds `List-Unsubscribe` and `List-Unsubscribe-Post` headers to rendered_body (RFC 8058)
- Appends unsubscribe link to HTML body if present
- Config: `{"base_url": "https://notifico.example.com"}` — defaults to empty (skip)
- Generates unsubscribe URL: `{base_url}/api/v1/public/unsubscribe?token={token}`

**Step 3: Tests**

- Test that HTML body gets unsubscribe link appended
- Test that List-Unsubscribe header is added to rendered_body
- Test that missing base_url config is a no-op

**Step 4: Register in main.rs**

Add `middleware_registry.register(Arc::new(UnsubscribeLinkMiddleware));`

**Step 5: Commit**

```
feat: add unsubscribe_link middleware
```

---

## Task 59: Click tracking middleware

**Files:**
- Create: `notifico-core/src/middleware/click_tracking.rs`

**Step 1: Implement ClickTrackingMiddleware**

Post-render middleware that:
- Scans HTML content for `href="..."` links
- Rewrites each URL to `{base_url}/t/click/{token}` where token encodes delivery_log_id + original URL
- Config: `{"base_url": "https://notifico.example.com"}`
- Skips mailto: and tel: links
- Skips unsubscribe links (already handled)

**Step 2: Tests**

- Test URL rewriting in HTML
- Test mailto/tel links are preserved
- Test missing base_url is a no-op

**Step 3: Register in main.rs**

**Step 4: Commit**

```
feat: add click_tracking middleware
```

---

## Task 60: Open tracking pixel middleware

**Files:**
- Create: `notifico-core/src/middleware/open_tracking.rs`

**Step 1: Implement OpenTrackingMiddleware**

Post-render middleware that:
- Appends `<img src="{base_url}/t/open/{token}" width="1" height="1" style="display:none" alt="" />` before `</body>` in HTML content
- Config: `{"base_url": "https://notifico.example.com"}`
- Only applies to content with HTML (email channel)

**Step 2: Tests**

- Test pixel is inserted before `</body>`
- Test non-HTML content is unchanged
- Test missing base_url is a no-op

**Step 3: Register in main.rs**

**Step 4: Commit**

```
feat: add open_tracking middleware
```

---

## Task 61: UTM parameters middleware

**Files:**
- Create: `notifico-core/src/middleware/utm_params.rs`

**Step 1: Implement UtmParamsMiddleware**

Post-render middleware that:
- Scans HTML content for `href="..."` links
- Appends UTM query parameters to each URL
- Default params: `utm_source=notifico`, `utm_medium={channel}`, `utm_campaign={event_name}`
- Config: `{"source": "notifico", "medium": null, "campaign": null}` — null means use defaults from context
- Skips mailto:, tel:, and anchor (#) links
- Preserves existing query params

**Step 2: Tests**

- Test UTM params appended to clean URLs
- Test UTM params appended to URLs with existing query params
- Test mailto/tel/anchor links preserved

**Step 3: Register in main.rs**

**Step 4: Commit**

```
feat: add utm_params middleware
```

---

## Task 62: Plaintext fallback middleware

**Files:**
- Create: `notifico-core/src/middleware/plaintext_fallback.rs`
- Modify: `notifico-core/Cargo.toml` — add html2text dependency

**Step 1: Add html2text to workspace deps**

Root `Cargo.toml`: `html2text = "0.14"`
`notifico-core/Cargo.toml`: `html2text = { workspace = true }`

**Step 2: Implement PlaintextFallbackMiddleware**

Post-render middleware that:
- Checks if rendered_body has `html` field but no `text` field
- Converts HTML to plain text via html2text
- Adds `text` field with the plain text version
- Config: `{"width": 80}` — text wrap width

**Step 3: Tests**

- Test HTML converts to plain text
- Test existing text field is not overwritten
- Test non-email content is unchanged

**Step 4: Register in main.rs**

**Step 5: Commit**

```
feat: add plaintext_fallback middleware
```

---

## Task 63: Update Dockerfile for OpenTelemetry

**Files:**
- Modify: `Dockerfile`
- Modify: `docker-compose.yml`

**Step 1: Add OTel env vars to Dockerfile**

Add `NOTIFICO_OTEL_ENDPOINT` env var (empty by default).

**Step 2: Add optional Jaeger/OTEL collector to docker-compose.yml**

Add a commented-out `jaeger` service for local development tracing.

**Step 3: Commit**

```
feat: update Docker config for OpenTelemetry support
```
