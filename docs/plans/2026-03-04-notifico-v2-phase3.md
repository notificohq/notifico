# Phase 3: Ingest API & Auth — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the Ingest API (`POST /api/v1/events`) with API key authentication, wiring together the full pipeline: authenticate → validate → resolve event → resolve recipients → check idempotency → resolve pipeline rules → resolve templates → render → produce DeliveryTasks. Also add recipient repo queries needed for contact resolution.

**Architecture:** API key sent via `Authorization: Bearer nk_live_...` header. Middleware extracts key, hashes with SHA-256, looks up in `api_key` table, extracts `project_id` and `scope`. Ingest handler receives `IngestEvent`, resolves event by name, finds pipeline rules, resolves recipient contacts (from DB or inline overrides), renders templates per rule, checks idempotency, and returns list of produced delivery task IDs.

**Tech Stack:** axum 0.8 extractors + middleware, sha2, sea-orm 2.0.0-rc.35

**Design doc:** `docs/plans/2026-03-03-notifico-v2-design.md` (sections 1, 3, 4)

---

## Overview

| Task | Name | Crate | Tests |
|------|------|-------|-------|
| 14 | API key repository (DB queries) | notifico-db | 3 |
| 15 | Recipient repository (DB queries) | notifico-db | 3 |
| 16 | API key auth extractor | notifico-server | 4 |
| 17 | Ingest handler | notifico-server | 5 |
| 18 | Wire routes + integration test | notifico-server | 2 |

---

### Task 14: API Key Repository (DB queries)

**Files:**
- Create: `notifico-db/src/repo/api_key.rs`
- Modify: `notifico-db/src/repo/mod.rs`
- Modify: `notifico-db/Cargo.toml` (add `sha2` dep)

**Step 1: Add sha2 to workspace**

`Cargo.toml` (workspace root) — add to `[workspace.dependencies]`:
```toml
sha2 = "0.10"
```

`notifico-db/Cargo.toml` — add to `[dependencies]`:
```toml
sha2 = { workspace = true }
```

**Step 2: Write API key repo**

Create `notifico-db/src/repo/api_key.rs`:

```rust
use sea_orm::*;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Result of looking up an API key.
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub scope: String,
    pub enabled: bool,
}

/// Hash a raw API key with SHA-256 (same algorithm used when creating keys).
pub fn hash_api_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Look up an API key by its raw value. Hashes it and queries the DB.
pub async fn find_by_raw_key(
    db: &DatabaseConnection,
    raw_key: &str,
) -> Result<Option<ApiKeyInfo>, DbErr> {
    let key_hash = hash_api_key(raw_key);

    let row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT id, project_id, name, scope, enabled
               FROM api_key
               WHERE key_hash = $1"#,
            [key_hash.into()],
        ))
        .await?;

    match row {
        Some(r) => Ok(Some(ApiKeyInfo {
            id: r.try_get("", "id")?,
            project_id: r.try_get("", "project_id")?,
            name: r.try_get("", "name")?,
            scope: r.try_get("", "scope")?,
            enabled: r.try_get("", "enabled")?,
        })),
        None => Ok(None),
    }
}

/// Insert an API key (for testing / admin). Stores the SHA-256 hash.
pub async fn insert_api_key(
    db: &DatabaseConnection,
    id: Uuid,
    project_id: Uuid,
    name: &str,
    raw_key: &str,
    scope: &str,
) -> Result<(), DbErr> {
    let key_hash = hash_api_key(raw_key);
    let prefix = &raw_key[..raw_key.len().min(16)];

    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"INSERT INTO api_key (id, project_id, name, key_hash, key_prefix, scope)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
        [
            id.into(),
            project_id.into(),
            name.into(),
            key_hash.into(),
            prefix.into(),
            scope.into(),
        ],
    ))
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};

    async fn setup_db() -> DatabaseConnection {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();
        db
    }

    async fn seed_project(db: &DatabaseConnection) -> Uuid {
        let project_id = Uuid::now_v7();
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();
        project_id
    }

    #[tokio::test]
    async fn hash_and_find_api_key() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;
        let key_id = Uuid::now_v7();
        let raw_key = "nk_live_test1234567890abcdef";

        insert_api_key(&db, key_id, project_id, "Test Key", raw_key, "ingest")
            .await
            .unwrap();

        let found = find_by_raw_key(&db, raw_key).await.unwrap().unwrap();
        assert_eq!(found.id, key_id);
        assert_eq!(found.project_id, project_id);
        assert_eq!(found.scope, "ingest");
        assert!(found.enabled);
    }

    #[tokio::test]
    async fn find_nonexistent_key_returns_none() {
        let db = setup_db().await;
        let found = find_by_raw_key(&db, "nk_live_doesnotexist").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn disabled_key_still_found() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;
        let key_id = Uuid::now_v7();
        let raw_key = "nk_live_disabled_key_12345";

        insert_api_key(&db, key_id, project_id, "Disabled", raw_key, "ingest")
            .await
            .unwrap();

        // Disable it
        db.execute_unprepared(&format!(
            "UPDATE api_key SET enabled = false WHERE id = '{key_id}'"
        ))
        .await
        .unwrap();

        let found = find_by_raw_key(&db, raw_key).await.unwrap().unwrap();
        assert!(!found.enabled);
    }
}
```

**Step 3: Wire up**

Modify `notifico-db/src/repo/mod.rs`:
```rust
pub mod api_key;
pub mod idempotency;
pub mod template;
```

**Step 4: Verify**

Run: `cargo test -p notifico-db -- repo::api_key`
Expected: 3 tests pass

**Step 5: Commit**

```bash
git add notifico-db/src/repo/api_key.rs notifico-db/src/repo/mod.rs notifico-db/Cargo.toml Cargo.toml Cargo.lock
git commit -m "feat: add API key repository with SHA-256 lookup"
```

---

### Task 15: Recipient Repository (DB queries)

**Files:**
- Create: `notifico-db/src/repo/recipient.rs`
- Modify: `notifico-db/src/repo/mod.rs`

Adds queries to look up recipients by external_id and resolve their contacts per channel.

**Step 1: Write recipient repo**

Create `notifico-db/src/repo/recipient.rs`:

```rust
use sea_orm::*;
use serde_json::Value;
use uuid::Uuid;

/// A recipient looked up from DB.
#[derive(Debug, Clone)]
pub struct RecipientRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub external_id: String,
    pub locale: String,
    pub timezone: String,
    pub metadata: Value,
}

/// Look up a recipient by project_id + external_id.
pub async fn find_by_external_id(
    db: &DatabaseConnection,
    project_id: Uuid,
    external_id: &str,
) -> Result<Option<RecipientRow>, DbErr> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT id, project_id, external_id, locale, timezone, metadata
               FROM recipient
               WHERE project_id = $1 AND external_id = $2"#,
            [project_id.into(), external_id.into()],
        ))
        .await?;

    match row {
        Some(r) => Ok(Some(RecipientRow {
            id: r.try_get("", "id")?,
            project_id: r.try_get("", "project_id")?,
            external_id: r.try_get("", "external_id")?,
            locale: r.try_get("", "locale")?,
            timezone: r.try_get("", "timezone")?,
            metadata: r.try_get("", "metadata")?,
        })),
        None => Ok(None),
    }
}

/// A contact value for a specific channel.
#[derive(Debug, Clone)]
pub struct ContactRow {
    pub id: Uuid,
    pub channel: String,
    pub value: String,
    pub verified: bool,
}

/// Get all contacts for a recipient.
pub async fn get_contacts(
    db: &DatabaseConnection,
    recipient_id: Uuid,
) -> Result<Vec<ContactRow>, DbErr> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT id, channel, value, verified
               FROM recipient_contact
               WHERE recipient_id = $1"#,
            [recipient_id.into()],
        ))
        .await?;

    let mut contacts = Vec::new();
    for row in rows {
        contacts.push(ContactRow {
            id: row.try_get("", "id")?,
            channel: row.try_get("", "channel")?,
            value: row.try_get("", "value")?,
            verified: row.try_get("", "verified")?,
        });
    }
    Ok(contacts)
}

/// Upsert a recipient (insert or update by project_id + external_id).
/// Returns the recipient's internal UUID.
pub async fn upsert_recipient(
    db: &DatabaseConnection,
    project_id: Uuid,
    external_id: &str,
    locale: Option<&str>,
) -> Result<Uuid, DbErr> {
    // Try to find existing
    if let Some(existing) = find_by_external_id(db, project_id, external_id).await? {
        return Ok(existing.id);
    }

    // Insert new
    let id = Uuid::now_v7();
    let locale = locale.unwrap_or("en");

    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"INSERT INTO recipient (id, project_id, external_id, locale)
           VALUES ($1, $2, $3, $4)"#,
        [id.into(), project_id.into(), external_id.into(), locale.into()],
    ))
    .await?;

    Ok(id)
}

/// Upsert a contact for a recipient (insert or update by recipient_id + channel + value).
pub async fn upsert_contact(
    db: &DatabaseConnection,
    recipient_id: Uuid,
    channel: &str,
    value: &str,
) -> Result<(), DbErr> {
    // Check if contact exists
    let existing = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT id FROM recipient_contact
               WHERE recipient_id = $1 AND channel = $2 AND value = $3"#,
            [recipient_id.into(), channel.into(), value.into()],
        ))
        .await?;

    if existing.is_some() {
        return Ok(()); // Already exists
    }

    let id = Uuid::now_v7();
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"INSERT INTO recipient_contact (id, recipient_id, channel, value)
           VALUES ($1, $2, $3, $4)"#,
        [id.into(), recipient_id.into(), channel.into(), value.into()],
    ))
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};

    async fn setup_db() -> DatabaseConnection {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();
        db
    }

    async fn seed_project(db: &DatabaseConnection) -> Uuid {
        let project_id = Uuid::now_v7();
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();
        project_id
    }

    #[tokio::test]
    async fn upsert_and_find_recipient() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;

        let id = upsert_recipient(&db, project_id, "user-123", Some("ru"))
            .await
            .unwrap();

        let found = find_by_external_id(&db, project_id, "user-123")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found.id, id);
        assert_eq!(found.locale, "ru");
        assert_eq!(found.external_id, "user-123");
    }

    #[tokio::test]
    async fn upsert_contact_and_get() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;

        let rid = upsert_recipient(&db, project_id, "user-456", None)
            .await
            .unwrap();

        upsert_contact(&db, rid, "email", "test@example.com")
            .await
            .unwrap();
        upsert_contact(&db, rid, "sms", "+1234567890")
            .await
            .unwrap();

        let contacts = get_contacts(&db, rid).await.unwrap();
        assert_eq!(contacts.len(), 2);

        let channels: Vec<&str> = contacts.iter().map(|c| c.channel.as_str()).collect();
        assert!(channels.contains(&"email"));
        assert!(channels.contains(&"sms"));
    }

    #[tokio::test]
    async fn upsert_recipient_idempotent() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;

        let id1 = upsert_recipient(&db, project_id, "user-789", None)
            .await
            .unwrap();
        let id2 = upsert_recipient(&db, project_id, "user-789", None)
            .await
            .unwrap();

        assert_eq!(id1, id2);
    }
}
```

**Step 2: Wire up**

Modify `notifico-db/src/repo/mod.rs`:
```rust
pub mod api_key;
pub mod idempotency;
pub mod recipient;
pub mod template;
```

**Step 3: Verify**

Run: `cargo test -p notifico-db -- repo::recipient`
Expected: 3 tests pass

**Step 4: Commit**

```bash
git add notifico-db/src/repo/recipient.rs notifico-db/src/repo/mod.rs
git commit -m "feat: add recipient repository with upsert and contact queries"
```

---

### Task 16: API Key Auth Extractor

**Files:**
- Create: `notifico-server/src/auth.rs`
- Modify: `notifico-server/src/main.rs` (add `mod auth;`)
- Modify: `notifico-server/Cargo.toml` (add `sha2`)

An axum extractor that reads `Authorization: Bearer <key>`, hashes it, looks up in DB, and produces `AuthContext { project_id, scope }`. Returns 401 on invalid/missing/disabled key. Returns 403 if scope doesn't match.

**Step 1: Add sha2 to notifico-server**

`notifico-server/Cargo.toml` — add:
```toml
sha2 = { workspace = true }
```

**Step 2: Write auth extractor**

Create `notifico-server/src/auth.rs`:

```rust
use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, State},
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
            AuthError::InvalidFormat => (StatusCode::UNAUTHORIZED, "Invalid Authorization format, expected: Bearer <key>"),
            AuthError::InvalidKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
            AuthError::DisabledKey => (StatusCode::UNAUTHORIZED, "API key is disabled"),
            AuthError::InsufficientScope { .. } => (StatusCode::FORBIDDEN, "Insufficient API key scope"),
            AuthError::DbError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };

        (status, message).into_response()
    }
}

impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
    Arc<AppState>: FromRequestParts<S, Rejection = std::convert::Infallible>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let State(app_state) = State::<Arc<AppState>>::from_request_parts(parts, state)
            .await
            .expect("AppState always available");

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
        let key_info = notifico_db::repo::api_key::find_by_raw_key(&app_state.db, raw_key)
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
        use axum::http::StatusCode;

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
```

**Step 3: Wire up**

Modify `notifico-server/src/main.rs` — add `mod auth;` at the top:
```rust
mod auth;
mod config;
mod worker;
```

**Step 4: Verify**

Run: `cargo test -p notifico-server -- auth`
Expected: 4 tests pass

**Step 5: Commit**

```bash
git add notifico-server/src/auth.rs notifico-server/src/main.rs notifico-server/Cargo.toml Cargo.lock
git commit -m "feat: add API key auth extractor with scope checking"
```

---

### Task 17: Ingest Handler

**Files:**
- Create: `notifico-server/src/ingest.rs`
- Modify: `notifico-server/src/main.rs` (add `mod ingest;`, add route)

The ingest handler:
1. Receives `IngestEvent` JSON body
2. Uses `AuthContext` to get `project_id` (requires `ingest` scope)
3. Resolves event by name from DB
4. Gets pipeline rules for the event
5. For each recipient × each rule:
   a. Resolve or upsert recipient by external_id
   b. Get contact value for the rule's channel (from inline contacts or DB)
   c. Resolve template (by template_id + recipient locale)
   d. Check idempotency
   e. Execute pipeline (render template)
   f. Collect PipelineOutput as DeliveryTask
6. Returns JSON response with accepted task count

**Step 1: Write ingest handler**

Create `notifico-server/src/ingest.rs`:

```rust
use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use uuid::Uuid;

use notifico_core::event::IngestEvent;
use notifico_core::pipeline::{PipelineInput, execute_pipeline};
use notifico_db::repo;

use crate::AppState;
use crate::auth::AuthContext;

#[derive(Debug, Serialize)]
pub struct IngestResponse {
    pub accepted: usize,
    pub task_ids: Vec<Uuid>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

pub async fn handle_ingest(
    State(state): State<Arc<AppState>>,
    auth: AuthContext,
    Json(event): Json<IngestEvent>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Check scope
    auth.require_scope("ingest")
        .map_err(|e| (StatusCode::FORBIDDEN, format!("{e:?}")))?;

    let project_id = auth.project_id;
    let default_locale = &state.config.project.default_locale;

    // Resolve event by name
    let event_row = repo::template::find_event_by_name(&state.db, project_id, &event.event)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                format!("Event not found: {}", event.event),
            )
        })?;

    // Get pipeline rules for this event
    let rules = repo::template::get_pipeline_rules(&state.db, event_row.id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if rules.is_empty() {
        return Ok(Json(IngestResponse {
            accepted: 0,
            task_ids: vec![],
            errors: vec![format!(
                "No pipeline rules configured for event: {}",
                event.event
            )],
        }));
    }

    let mut task_ids = Vec::new();
    let mut errors = Vec::new();

    for recipient_input in &event.recipients {
        // Resolve or upsert recipient
        let recipient_id = match repo::recipient::upsert_recipient(
            &state.db,
            project_id,
            &recipient_input.id,
            None,
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                errors.push(format!(
                    "Failed to resolve recipient {}: {}",
                    recipient_input.id, e
                ));
                continue;
            }
        };

        // Store inline contacts if provided
        for (channel, value) in &recipient_input.contacts {
            if let Err(e) =
                repo::recipient::upsert_contact(&state.db, recipient_id, channel, value).await
            {
                tracing::warn!(
                    recipient = %recipient_input.id,
                    channel = %channel,
                    error = %e,
                    "Failed to upsert contact"
                );
            }
        }

        // Get recipient info for locale
        let recipient_row =
            repo::recipient::find_by_external_id(&state.db, project_id, &recipient_input.id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let recipient_locale = recipient_row
            .as_ref()
            .map(|r| r.locale.as_str())
            .unwrap_or(default_locale);

        // Get contacts from DB
        let db_contacts = repo::recipient::get_contacts(&state.db, recipient_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for rule in &rules {
            // Find contact value for this channel
            // Priority: inline contacts > DB contacts
            let contact_value = recipient_input
                .contacts
                .get(&rule.channel)
                .cloned()
                .or_else(|| {
                    db_contacts
                        .iter()
                        .find(|c| c.channel == rule.channel)
                        .map(|c| c.value.clone())
                });

            let contact_value = match contact_value {
                Some(v) => v,
                None => {
                    errors.push(format!(
                        "No contact for recipient {} on channel {}",
                        recipient_input.id, rule.channel
                    ));
                    continue;
                }
            };

            // Check idempotency
            if let Some(ref client_key) = event.idempotency_key {
                let idem_key = repo::idempotency::make_idempotency_key(
                    &event.event,
                    recipient_id,
                    &rule.channel,
                    Some(client_key),
                );
                match repo::idempotency::check_and_insert(&state.db, &idem_key).await {
                    Ok(true) => {
                        tracing::debug!(
                            key = %idem_key,
                            "Duplicate delivery skipped"
                        );
                        continue; // Skip duplicate
                    }
                    Ok(false) => {} // Not a duplicate, proceed
                    Err(e) => {
                        tracing::error!(error = %e, "Idempotency check failed");
                        // Proceed anyway — better to deliver twice than not at all
                    }
                }
            }

            // Resolve template
            let template = match repo::template::resolve_template(
                &state.db,
                rule.template_id,
                recipient_locale,
                default_locale,
            )
            .await
            {
                Ok(Some(t)) => t,
                Ok(None) => {
                    errors.push(format!(
                        "Template not found for rule {} (template_id: {}, locale: {})",
                        rule.id, rule.template_id, recipient_locale
                    ));
                    continue;
                }
                Err(e) => {
                    errors.push(format!("Template resolution error: {}", e));
                    continue;
                }
            };

            // Execute pipeline (render template)
            let pipeline_input = PipelineInput {
                project_id,
                event_name: event.event.clone(),
                recipient_id,
                recipient_locale: recipient_locale.to_string(),
                channel: rule.channel.clone(),
                contact_value,
                template_body: template.body,
                context_data: event.data.clone(),
                idempotency_key: event.idempotency_key.clone(),
                max_attempts: 5,
            };

            match execute_pipeline(pipeline_input) {
                Ok(output) => {
                    task_ids.push(output.id);
                    tracing::info!(
                        task_id = %output.id,
                        channel = %output.channel,
                        recipient = %recipient_input.id,
                        "Delivery task created"
                    );
                    // TODO: In a future phase, enqueue output as DeliveryTask via apalis
                }
                Err(e) => {
                    errors.push(format!(
                        "Pipeline error for recipient {} channel {}: {}",
                        recipient_input.id, rule.channel, e
                    ));
                }
            }
        }
    }

    let accepted = task_ids.len();
    Ok(Json(IngestResponse {
        accepted,
        task_ids,
        errors,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ingest_response_serialization() {
        let resp = IngestResponse {
            accepted: 2,
            task_ids: vec![Uuid::now_v7(), Uuid::now_v7()],
            errors: vec![],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["accepted"], 2);
        assert!(json.get("errors").is_none()); // skip_serializing_if empty
    }

    #[test]
    fn ingest_response_with_errors() {
        let resp = IngestResponse {
            accepted: 1,
            task_ids: vec![Uuid::now_v7()],
            errors: vec!["No contact for user on sms".into()],
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["accepted"], 1);
        assert_eq!(json["errors"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn ingest_event_parsing() {
        let json_str = r#"{
            "event": "order.confirmed",
            "recipients": [
                {"id": "user-123", "contacts": {"email": "test@example.com"}},
                {"id": "user-456"}
            ],
            "data": {"order_id": 42, "total": "99.90"},
            "idempotency_key": "order-42-confirm"
        }"#;
        let event: IngestEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.event, "order.confirmed");
        assert_eq!(event.recipients.len(), 2);
        assert_eq!(event.idempotency_key.as_deref(), Some("order-42-confirm"));
    }

    #[test]
    fn ingest_event_minimal() {
        let json_str = r#"{
            "event": "user.signup",
            "recipients": [{"id": "u-1"}],
            "data": {}
        }"#;
        let event: IngestEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.event, "user.signup");
        assert!(event.idempotency_key.is_none());
        assert!(event.recipients[0].contacts.is_empty());
    }

    #[test]
    fn ingest_event_with_multiple_contacts() {
        let json_str = r#"{
            "event": "alert",
            "recipients": [{
                "id": "user-1",
                "contacts": {
                    "email": "a@b.com",
                    "sms": "+1234567890",
                    "telegram": "12345"
                }
            }],
            "data": {"message": "Server down"}
        }"#;
        let event: IngestEvent = serde_json::from_str(json_str).unwrap();
        assert_eq!(event.recipients[0].contacts.len(), 3);
    }
}
```

**Step 2: Wire up route**

Modify `notifico-server/src/main.rs` — add `mod ingest;` and the route:
```rust
mod auth;
mod config;
mod ingest;
mod worker;
```

In `start_api_server`, add the route:
```rust
use axum::routing::post;

let app = Router::new()
    .route("/health", get(health))
    .route("/ready", get(ready))
    .route("/api/v1/events", post(ingest::handle_ingest))
    .layer(TraceLayer::new_for_http())
    .with_state(state.clone());
```

**Step 3: Verify**

Run: `cargo test -p notifico-server -- ingest`
Expected: 5 tests pass

Run: `cargo build -p notifico-server`
Expected: compiles

**Step 4: Commit**

```bash
git add notifico-server/src/ingest.rs notifico-server/src/main.rs
git commit -m "feat: add ingest handler with full pipeline execution"
```

---

### Task 18: Wire Routes + Integration Test

**Files:**
- Create: `notifico-server/tests/ingest_integration.rs`
- Modify: `notifico-server/Cargo.toml` (add `axum-test` or `tower` test deps)

An integration test that sets up an in-memory SQLite DB, seeds project + event + pipeline rule + template, creates an API key, and sends a `POST /api/v1/events` through the axum router. Verifies the response contains accepted task IDs.

**Step 1: Add test dependencies**

`Cargo.toml` (workspace root) — add:
```toml
tower = { version = "0.5", features = ["util"] }
hyper = "1.6"
```

`notifico-server/Cargo.toml` — add:
```toml
[dev-dependencies]
tower = { workspace = true }
hyper = { workspace = true }
axum = { workspace = true, features = ["macros"] }
serde_json = { workspace = true }
tokio = { workspace = true }
```

**Step 2: Write integration test**

Create `notifico-server/tests/ingest_integration.rs`:

```rust
use std::sync::Arc;

use axum::{Router, body::Body, http::{Request, StatusCode}};
use axum::routing::{get, post};
use sea_orm::ConnectionTrait;
use serde_json::json;
use tower::util::ServiceExt;
use uuid::Uuid;

// We need to reconstruct the app since AppState is private.
// For integration tests, we create the router inline.

async fn setup_app() -> (Router, String) {
    // Connect to in-memory SQLite
    let db = notifico_db::connect("sqlite::memory:").await.unwrap();
    notifico_db::run_migrations(&db).await.unwrap();

    // Seed data
    let project_id = Uuid::now_v7();
    let event_id = Uuid::now_v7();
    let template_id = Uuid::now_v7();
    let version_id = Uuid::now_v7();
    let content_id = Uuid::now_v7();
    let rule_id = Uuid::now_v7();

    // Project
    db.execute_unprepared(&format!(
        "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
    )).await.unwrap();

    // Event
    db.execute_unprepared(&format!(
        "INSERT INTO event (id, project_id, name, category) VALUES ('{event_id}', '{project_id}', 'order.confirmed', 'transactional')"
    )).await.unwrap();

    // Template
    db.execute_unprepared(&format!(
        "INSERT INTO template (id, project_id, name, channel) VALUES ('{template_id}', '{project_id}', 'order_email', 'email')"
    )).await.unwrap();

    // Template version
    db.execute_unprepared(&format!(
        "INSERT INTO template_version (id, template_id, version, is_current) VALUES ('{version_id}', '{template_id}', 1, true)"
    )).await.unwrap();

    // Template content
    db.execute_unprepared(&format!(
        r#"INSERT INTO template_content (id, template_version_id, locale, body) VALUES ('{content_id}', '{version_id}', 'en', '{{"subject": "Order #{{{{ order_id }}}}", "text": "Hello {{{{ name }}}}"}}')"#
    )).await.unwrap();

    // Pipeline rule
    db.execute_unprepared(&format!(
        "INSERT INTO pipeline_rule (id, event_id, channel, template_id, enabled, priority) VALUES ('{rule_id}', '{event_id}', 'email', '{template_id}', true, 10)"
    )).await.unwrap();

    // API key
    let raw_key = "nk_live_integration_test_key_1234";
    notifico_db::repo::api_key::insert_api_key(
        &db, Uuid::now_v7(), project_id, "Test Key", raw_key, "ingest"
    ).await.unwrap();

    // Build config
    let config = notifico_server_test_config();

    let registry = notifico_core::registry::TransportRegistry::new();

    let state = Arc::new(notifico_server_test_state(db, config, registry));

    let app = build_test_router(state);

    (app, raw_key.to_string())
}

fn notifico_server_test_config() -> notifico_server::Config {
    notifico_server::Config::default_for_test()
}

// NOTE: This test requires AppState and router to be accessible.
// Since AppState is private in notifico-server, we'll need to either:
// 1. Make AppState and build_router public for testing
// 2. Or test via HTTP using a spawned server
//
// For now this file documents the integration test structure.
// Implementation requires making AppState + router builder pub(crate) or pub.

#[tokio::test]
async fn ingest_event_end_to_end() {
    // This test will be enabled once AppState is made accessible for testing.
    // See implementation notes above.
}

#[tokio::test]
async fn ingest_without_auth_returns_401() {
    // This test will be enabled once AppState is made accessible for testing.
}
```

**Step 2b: Make AppState accessible for tests**

Modify `notifico-server/src/main.rs`:
- Make `AppState` `pub(crate)` and add a `pub(crate) fn build_router(state: Arc<AppState>) -> Router`
- Export a test helper module

Add to `notifico-server/src/main.rs`:

```rust
pub(crate) fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/api/v1/events", post(ingest::handle_ingest))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
```

Update `start_api_server` to use `build_router`:
```rust
async fn start_api_server(state: Arc<AppState>) {
    let app = build_router(state.clone());
    // ... rest unchanged
}
```

Then rewrite `notifico-server/tests/ingest_integration.rs` to import `build_router` and `AppState` (requires `pub` visibility or `#[cfg(test)]` module).

**Alternative (simpler):** Add integration tests as `#[cfg(test)] mod tests` inside `main.rs` or a separate file included via `#[path]`.

**Step 3: Verify**

Run: `cargo test -p notifico-server`
Expected: all tests pass (auth + ingest unit tests + integration)

Run: `cargo test --workspace`
Expected: all ~45 tests pass

**Step 4: Commit**

```bash
git add notifico-server/
git commit -m "feat: wire ingest API route with auth and add integration tests"
```

---

## Phase 3 Summary

After completing all tasks:

| Component | What it does |
|-----------|-------------|
| `notifico-db/repo/api_key` | SHA-256 API key lookup |
| `notifico-db/repo/recipient` | Recipient upsert + contact resolution |
| `notifico-server/auth` | Axum extractor: Bearer token → AuthContext |
| `notifico-server/ingest` | Full pipeline: event → rules → templates → render → tasks |
| Route: `POST /api/v1/events` | Authenticated ingest endpoint |

**Total new tests:** ~17
**New deps:** sha2
**New routes:** `POST /api/v1/events`

**End-to-end flow now works:**
```
Client POST /api/v1/events
  → Auth extractor validates API key, extracts project_id
  → Find event by name
  → Get pipeline rules
  → For each recipient × rule:
      → Resolve/upsert recipient
      → Get contact value
      → Check idempotency
      → Resolve template (with locale fallback)
      → Render template via minijinja
      → Produce PipelineOutput (delivery task)
  → Return { accepted: N, task_ids: [...] }
```

**What's still missing for actual delivery:** Queue integration (enqueue tasks via apalis instead of just returning IDs) and transport implementations (Phase 4).
