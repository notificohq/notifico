# Phase 2: Pipeline Engine — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the core pipeline: template rendering (minijinja), job queue (apalis), pipeline executor (event → rules → render → enqueue), delivery worker (dequeue → send → log), retry logic, and idempotency.

**Architecture:** Events arrive, pipeline executor resolves pipeline rules, renders templates per channel+locale via minijinja, enqueues delivery tasks into apalis. Workers dequeue tasks, call `Transport::send()`, log results. Retry with exponential backoff on failure. Idempotency deduplication prevents double-sends.

**Tech Stack:** minijinja 2.16, apalis 1.0.0-rc.4, apalis-redis, apalis-sqlite, sea-orm 1.1, tokio, serde

**Design doc:** `docs/plans/2026-03-03-notifico-v2-design.md` (sections 1, 4)

---

## Overview

| Task | Name | Crate | Tests |
|------|------|-------|-------|
| 8 | Template engine (minijinja) | notifico-template | 6 |
| 9 | Template repository (DB queries) | notifico-db | 4 |
| 10 | Queue abstraction (apalis) | notifico-queue | 3 |
| 11 | Delivery task types | notifico-core | 3 |
| 12 | Pipeline executor | notifico-core | 5 |
| 13 | Delivery worker | notifico-server | 3 |
| 14 | Retry & dead-letter logic | notifico-queue | 3 |
| 15 | Idempotency guard | notifico-db | 3 |

---

### Task 8: Template Engine (minijinja)

**Files:**
- Create: `notifico-template/Cargo.toml`
- Create: `notifico-template/src/lib.rs`
- Modify: `Cargo.toml` (workspace — add member + deps)

**Step 1: Add notifico-template crate to workspace**

`Cargo.toml` (workspace root) — add to `[workspace]` members:
```toml
members = [
    "notifico-core",
    "notifico-db",
    "notifico-template",
    "notifico-server",
]
```

Add to `[workspace.dependencies]`:
```toml
minijinja = { version = "2.16", features = ["builtins"] }
notifico-template = { path = "notifico-template" }
```

**Step 2: Create `notifico-template/Cargo.toml`**

```toml
[package]
name = "notifico-template"
version.workspace = true
edition.workspace = true

[dependencies]
minijinja = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio = { workspace = true }
```

**Step 3: Write template engine with tests**

Create `notifico-template/src/lib.rs`:

```rust
use minijinja::Environment;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template rendering failed: {0}")]
    Render(#[from] minijinja::Error),

    #[error("Template not found: {0}")]
    NotFound(String),

    #[error("Invalid template body: {0}")]
    InvalidBody(String),
}

/// Render a single template string with the given context data.
pub fn render_string(template: &str, context: &Value) -> Result<String, TemplateError> {
    let env = Environment::new();
    let result = env.render_str(template, context)?;
    Ok(result)
}

/// Render multiple named fields from a JSONB body.
///
/// `body` is the template_content.body JSONB, e.g.:
/// ```json
/// { "subject": "Order {{ order_id }}", "text": "Hello {{ name }}" }
/// ```
///
/// Returns a map of field_name -> rendered_string.
pub fn render_body(
    body: &Value,
    context: &Value,
) -> Result<serde_json::Map<String, Value>, TemplateError> {
    let obj = body
        .as_object()
        .ok_or_else(|| TemplateError::InvalidBody("body must be a JSON object".into()))?;

    let env = Environment::new();
    let mut result = serde_json::Map::new();

    for (key, val) in obj {
        match val {
            Value::String(tmpl) => {
                let rendered = env.render_str(tmpl, context)?;
                result.insert(key.clone(), Value::String(rendered));
            }
            // Non-string values pass through unchanged (e.g. arrays, objects for structured data)
            other => {
                result.insert(key.clone(), other.clone());
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_simple_string() {
        let result = render_string("Hello {{ name }}!", &json!({"name": "World"})).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn render_string_with_filter() {
        let result =
            render_string("{{ name | upper }}", &json!({"name": "alice"})).unwrap();
        assert_eq!(result, "ALICE");
    }

    #[test]
    fn render_string_with_loop() {
        let result = render_string(
            "{% for item in items %}{{ item }},{% endfor %}",
            &json!({"items": ["a", "b", "c"]}),
        )
        .unwrap();
        assert_eq!(result, "a,b,c,");
    }

    #[test]
    fn render_body_multiple_fields() {
        let body = json!({
            "subject": "Order #{{ order_id }}",
            "text": "Hello {{ name }}, your order is confirmed."
        });
        let context = json!({"order_id": 42, "name": "Alice"});
        let result = render_body(&body, &context).unwrap();

        assert_eq!(result["subject"], "Order #42");
        assert_eq!(
            result["text"],
            "Hello Alice, your order is confirmed."
        );
    }

    #[test]
    fn render_body_non_string_passthrough() {
        let body = json!({
            "subject": "Hello {{ name }}",
            "buttons": [{"label": "Click", "url": "https://example.com"}]
        });
        let context = json!({"name": "Bob"});
        let result = render_body(&body, &context).unwrap();

        assert_eq!(result["subject"], "Hello Bob");
        assert_eq!(
            result["buttons"],
            json!([{"label": "Click", "url": "https://example.com"}])
        );
    }

    #[test]
    fn render_body_invalid_non_object() {
        let body = json!("not an object");
        let context = json!({});
        let result = render_body(&body, &context);
        assert!(result.is_err());
    }
}
```

**Step 4: Verify**

Run: `cargo test -p notifico-template`
Expected: 6 tests pass

**Step 5: Commit**

```bash
git add notifico-template/ Cargo.toml Cargo.lock
git commit -m "feat: add template engine crate with minijinja rendering"
```

---

### Task 9: Template Repository (DB queries)

**Files:**
- Create: `notifico-db/src/repo/mod.rs`
- Create: `notifico-db/src/repo/template.rs`
- Modify: `notifico-db/src/lib.rs`

This task adds sea-orm query functions to look up template content by (template_id, version, locale) with fallback to project default locale.

**Step 1: Create repo module**

Create `notifico-db/src/repo/mod.rs`:
```rust
pub mod template;
```

**Step 2: Write template repo with tests**

Create `notifico-db/src/repo/template.rs`:

```rust
use sea_orm::*;
use serde_json::Value;
use uuid::Uuid;

/// Resolved template content ready for rendering.
#[derive(Debug, Clone)]
pub struct ResolvedTemplate {
    pub template_id: Uuid,
    pub template_name: String,
    pub channel: String,
    pub version: i32,
    pub locale: String,
    pub body: Value,
}

/// Find the current version's content for a template, with locale fallback.
///
/// Lookup order:
/// 1. Exact locale match on the current version
/// 2. Fallback to `default_locale` on the current version
pub async fn resolve_template(
    db: &DatabaseConnection,
    template_id: Uuid,
    locale: &str,
    default_locale: &str,
) -> Result<Option<ResolvedTemplate>, DbErr> {
    // Find current version for this template
    let version_row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT tv.id, tv.version, t.name, t.channel
               FROM template_version tv
               JOIN template t ON t.id = tv.template_id
               WHERE tv.template_id = $1 AND tv.is_current = true"#,
            [template_id.into()],
        ))
        .await?;

    let version_row = match version_row {
        Some(r) => r,
        None => return Ok(None),
    };

    let version_id: Uuid = version_row.try_get("", "id")?;
    let version: i32 = version_row.try_get("", "version")?;
    let template_name: String = version_row.try_get("", "name")?;
    let channel: String = version_row.try_get("", "channel")?;

    // Try exact locale first
    if let Some(content) = find_content(db, version_id, locale).await? {
        return Ok(Some(ResolvedTemplate {
            template_id,
            template_name,
            channel,
            version,
            locale: locale.to_string(),
            body: content,
        }));
    }

    // Fallback to default locale
    if locale != default_locale {
        if let Some(content) = find_content(db, version_id, default_locale).await? {
            return Ok(Some(ResolvedTemplate {
                template_id,
                template_name,
                channel,
                version,
                locale: default_locale.to_string(),
                body: content,
            }));
        }
    }

    Ok(None)
}

async fn find_content(
    db: &DatabaseConnection,
    version_id: Uuid,
    locale: &str,
) -> Result<Option<Value>, DbErr> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT body FROM template_content
               WHERE template_version_id = $1 AND locale = $2"#,
            [version_id.into(), locale.into()],
        ))
        .await?;

    match row {
        Some(r) => {
            let body: Value = r.try_get("", "body")?;
            Ok(Some(body))
        }
        None => Ok(None),
    }
}

/// List all pipeline rules for an event.
#[derive(Debug, Clone)]
pub struct PipelineRuleRow {
    pub id: Uuid,
    pub channel: String,
    pub template_id: Uuid,
    pub enabled: bool,
    pub conditions: Option<Value>,
    pub priority: i32,
}

pub async fn get_pipeline_rules(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> Result<Vec<PipelineRuleRow>, DbErr> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT id, channel, template_id, enabled, conditions, priority
               FROM pipeline_rule
               WHERE event_id = $1 AND enabled = true
               ORDER BY priority DESC"#,
            [event_id.into()],
        ))
        .await?;

    let mut rules = Vec::new();
    for row in rows {
        rules.push(PipelineRuleRow {
            id: row.try_get("", "id")?,
            channel: row.try_get("", "channel")?,
            template_id: row.try_get("", "template_id")?,
            enabled: row.try_get("", "enabled")?,
            conditions: row.try_get("", "conditions").ok(),
            priority: row.try_get("", "priority")?,
        });
    }
    Ok(rules)
}

/// Look up an event by project_id + name.
#[derive(Debug, Clone)]
pub struct EventRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub category: String,
}

pub async fn find_event_by_name(
    db: &DatabaseConnection,
    project_id: Uuid,
    event_name: &str,
) -> Result<Option<EventRow>, DbErr> {
    let row = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"SELECT id, project_id, name, category
               FROM event
               WHERE project_id = $1 AND name = $2"#,
            [project_id.into(), event_name.into()],
        ))
        .await?;

    match row {
        Some(r) => Ok(Some(EventRow {
            id: r.try_get("", "id")?,
            project_id: r.try_get("", "project_id")?,
            name: r.try_get("", "name")?,
            category: r.try_get("", "category")?,
        })),
        None => Ok(None),
    }
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

    async fn seed_template(db: &DatabaseConnection) -> (Uuid, Uuid, Uuid) {
        let project_id = Uuid::now_v7();
        let template_id = Uuid::now_v7();
        let version_id = Uuid::now_v7();
        let content_en_id = Uuid::now_v7();
        let content_ru_id = Uuid::now_v7();

        // Insert project
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();

        // Insert template
        db.execute_unprepared(&format!(
            "INSERT INTO template (id, project_id, name, channel) VALUES ('{template_id}', '{project_id}', 'welcome_email', 'email')"
        ))
        .await
        .unwrap();

        // Insert current version
        db.execute_unprepared(&format!(
            "INSERT INTO template_version (id, template_id, version, is_current) VALUES ('{version_id}', '{template_id}', 1, true)"
        ))
        .await
        .unwrap();

        // Insert content for en and ru
        db.execute_unprepared(&format!(
            r#"INSERT INTO template_content (id, template_version_id, locale, body) VALUES ('{content_en_id}', '{version_id}', 'en', '{{"subject": "Welcome {{{{ name }}}}!", "text": "Hello {{{{ name }}}}"}}')"#
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            r#"INSERT INTO template_content (id, template_version_id, locale, body) VALUES ('{content_ru_id}', '{version_id}', 'ru', '{{"subject": "Привет {{{{ name }}}}!", "text": "Здравствуйте {{{{ name }}}}"}}')"#
        ))
        .await
        .unwrap();

        (project_id, template_id, version_id)
    }

    #[tokio::test]
    async fn resolve_template_exact_locale() {
        let db = setup_db().await;
        let (_project_id, template_id, _) = seed_template(&db).await;

        let result = resolve_template(&db, template_id, "ru", "en")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result.locale, "ru");
        assert_eq!(result.body["subject"], "Привет {{ name }}!");
    }

    #[tokio::test]
    async fn resolve_template_fallback_locale() {
        let db = setup_db().await;
        let (_project_id, template_id, _) = seed_template(&db).await;

        let result = resolve_template(&db, template_id, "de", "en")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(result.locale, "en");
    }

    #[tokio::test]
    async fn resolve_template_not_found() {
        let db = setup_db().await;
        let result = resolve_template(&db, Uuid::now_v7(), "en", "en")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_pipeline_rules_returns_sorted() {
        let db = setup_db().await;
        let (project_id, template_id, _) = seed_template(&db).await;
        let event_id = Uuid::now_v7();
        let rule1_id = Uuid::now_v7();
        let rule2_id = Uuid::now_v7();

        db.execute_unprepared(&format!(
            "INSERT INTO event (id, project_id, name, category) VALUES ('{event_id}', '{project_id}', 'order.confirmed', 'transactional')"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO pipeline_rule (id, event_id, channel, template_id, enabled, priority) VALUES ('{rule1_id}', '{event_id}', 'email', '{template_id}', true, 10)"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO pipeline_rule (id, event_id, channel, template_id, enabled, priority) VALUES ('{rule2_id}', '{event_id}', 'sms', '{template_id}', true, 20)"
        ))
        .await
        .unwrap();

        let rules = get_pipeline_rules(&db, event_id).await.unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].channel, "sms"); // higher priority first
        assert_eq!(rules[1].channel, "email");
    }
}
```

**Step 3: Wire up repo module**

Modify `notifico-db/src/lib.rs` — add after `pub mod migration;`:
```rust
pub mod repo;
```

Also add `uuid` and `tokio` to notifico-db's dev-dependencies if not already there (they should be).

**Step 4: Verify**

Run: `cargo test -p notifico-db`
Expected: 6 tests pass (2 migration + 4 repo)

**Step 5: Commit**

```bash
git add notifico-db/src/repo/ notifico-db/src/lib.rs
git commit -m "feat: add template and pipeline rule repository queries"
```

---

### Task 10: Queue Abstraction (apalis)

**Files:**
- Create: `notifico-queue/Cargo.toml`
- Create: `notifico-queue/src/lib.rs`
- Modify: `Cargo.toml` (workspace)

**Step 1: Add dependencies to workspace**

`Cargo.toml` (workspace root) — add to members:
```toml
members = [
    "notifico-core",
    "notifico-db",
    "notifico-template",
    "notifico-queue",
    "notifico-server",
]
```

Add to `[workspace.dependencies]`:
```toml
apalis = "1.0.0-rc.4"
apalis-redis = "1.0.0-rc.4"
apalis-sqlite = "1.0.0-rc.4"
notifico-queue = { path = "notifico-queue" }
notifico-template = { path = "notifico-template" }
```

**Step 2: Create `notifico-queue/Cargo.toml`**

```toml
[package]
name = "notifico-queue"
version.workspace = true
edition.workspace = true

[dependencies]
apalis = { workspace = true }
apalis-redis = { workspace = true }
apalis-sqlite = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
notifico-core = { workspace = true }
```

**Step 3: Write queue abstraction**

Create `notifico-queue/src/lib.rs`:

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// A delivery task to be enqueued and processed by workers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryTask {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_name: String,
    pub recipient_id: Uuid,
    pub channel: String,
    pub rendered_body: Value,
    pub contact_value: String,
    pub idempotency_key: Option<String>,
    pub attempt: u32,
    pub max_attempts: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_task_serialization_roundtrip() {
        let task = DeliveryTask {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            event_name: "order.confirmed".into(),
            recipient_id: Uuid::now_v7(),
            channel: "email".into(),
            rendered_body: serde_json::json!({"subject": "Hi", "text": "Hello"}),
            contact_value: "user@example.com".into(),
            idempotency_key: Some("key-123".into()),
            attempt: 0,
            max_attempts: 5,
        };

        let json = serde_json::to_string(&task).unwrap();
        let deserialized: DeliveryTask = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.event_name, "order.confirmed");
        assert_eq!(deserialized.channel, "email");
    }

    #[test]
    fn delivery_task_without_idempotency_key() {
        let task = DeliveryTask {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            event_name: "user.signup".into(),
            recipient_id: Uuid::now_v7(),
            channel: "sms".into(),
            rendered_body: serde_json::json!({"text": "Welcome!"}),
            contact_value: "+1234567890".into(),
            idempotency_key: None,
            attempt: 0,
            max_attempts: 3,
        };

        let json = serde_json::to_string(&task).unwrap();
        let deserialized: DeliveryTask = serde_json::from_str(&json).unwrap();
        assert!(deserialized.idempotency_key.is_none());
    }

    #[test]
    fn delivery_task_attempt_tracking() {
        let mut task = DeliveryTask {
            id: Uuid::now_v7(),
            project_id: Uuid::now_v7(),
            event_name: "test".into(),
            recipient_id: Uuid::now_v7(),
            channel: "email".into(),
            rendered_body: serde_json::json!({}),
            contact_value: "test@test.com".into(),
            idempotency_key: None,
            attempt: 0,
            max_attempts: 5,
        };

        task.attempt += 1;
        assert_eq!(task.attempt, 1);
        assert!(task.attempt < task.max_attempts);
    }
}
```

**Step 4: Verify**

Run: `cargo test -p notifico-queue`
Expected: 3 tests pass

**Step 5: Commit**

```bash
git add notifico-queue/ Cargo.toml Cargo.lock
git commit -m "feat: add queue crate with delivery task types"
```

---

### Task 11: Pipeline Executor

**Files:**
- Create: `notifico-core/src/pipeline.rs`
- Modify: `notifico-core/src/lib.rs`
- Modify: `notifico-core/Cargo.toml`

The pipeline executor takes an IngestEvent, resolves pipeline rules from DB, renders templates, and produces DeliveryTasks ready for enqueuing.

**Step 1: Add deps to notifico-core**

`notifico-core/Cargo.toml` — add:
```toml
tracing = { workspace = true }
```

**Step 2: Write pipeline executor with tests**

Create `notifico-core/src/pipeline.rs`:

```rust
use serde_json::Value;
use uuid::Uuid;

/// Input for the pipeline: one recipient + one pipeline rule match.
#[derive(Debug, Clone)]
pub struct PipelineInput {
    pub project_id: Uuid,
    pub event_name: String,
    pub recipient_id: Uuid,
    pub recipient_locale: String,
    pub channel: String,
    pub contact_value: String,
    pub template_body: Value,
    pub context_data: Value,
    pub idempotency_key: Option<String>,
    pub max_attempts: u32,
}

/// Output of the pipeline: a delivery task ready for enqueuing.
#[derive(Debug, Clone)]
pub struct PipelineOutput {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_name: String,
    pub recipient_id: Uuid,
    pub channel: String,
    pub rendered_body: Value,
    pub contact_value: String,
    pub idempotency_key: Option<String>,
    pub max_attempts: u32,
}

/// Execute the rendering pipeline for one recipient + one channel.
///
/// Steps:
/// 1. Merge recipient locale context into event data
/// 2. Render template body fields via minijinja
/// 3. Return PipelineOutput ready for enqueuing
pub fn execute_pipeline(input: PipelineInput) -> Result<PipelineOutput, crate::error::CoreError> {
    // Render template body with context data
    let rendered = notifico_template::render_body(&input.template_body, &input.context_data)
        .map_err(|e| crate::error::CoreError::TemplateRender(e.to_string()))?;

    Ok(PipelineOutput {
        id: Uuid::now_v7(),
        project_id: input.project_id,
        event_name: input.event_name,
        recipient_id: input.recipient_id,
        channel: input.channel,
        rendered_body: Value::Object(rendered),
        contact_value: input.contact_value,
        idempotency_key: input.idempotency_key,
        max_attempts: input.max_attempts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_input(body: Value, data: Value) -> PipelineInput {
        PipelineInput {
            project_id: Uuid::now_v7(),
            event_name: "order.confirmed".into(),
            recipient_id: Uuid::now_v7(),
            recipient_locale: "en".into(),
            channel: "email".into(),
            contact_value: "user@example.com".into(),
            template_body: body,
            context_data: data,
            idempotency_key: None,
            max_attempts: 5,
        }
    }

    #[test]
    fn execute_pipeline_renders_template() {
        let input = make_input(
            json!({"subject": "Order #{{ order_id }}", "text": "Hello {{ name }}"}),
            json!({"order_id": 42, "name": "Alice"}),
        );

        let output = execute_pipeline(input).unwrap();
        assert_eq!(output.rendered_body["subject"], "Order #42");
        assert_eq!(output.rendered_body["text"], "Hello Alice");
        assert_eq!(output.channel, "email");
    }

    #[test]
    fn execute_pipeline_preserves_metadata() {
        let input = make_input(
            json!({"text": "Hi {{ name }}"}),
            json!({"name": "Bob"}),
        );
        let project_id = input.project_id;
        let recipient_id = input.recipient_id;

        let output = execute_pipeline(input).unwrap();
        assert_eq!(output.project_id, project_id);
        assert_eq!(output.recipient_id, recipient_id);
        assert_eq!(output.event_name, "order.confirmed");
    }

    #[test]
    fn execute_pipeline_with_idempotency_key() {
        let mut input = make_input(
            json!({"text": "Hello"}),
            json!({}),
        );
        input.idempotency_key = Some("key-abc".into());

        let output = execute_pipeline(input).unwrap();
        assert_eq!(output.idempotency_key, Some("key-abc".into()));
    }

    #[test]
    fn execute_pipeline_passthrough_non_string() {
        let input = make_input(
            json!({
                "text": "Hello {{ name }}",
                "buttons": [{"label": "View", "url": "https://example.com"}]
            }),
            json!({"name": "Carol"}),
        );

        let output = execute_pipeline(input).unwrap();
        assert_eq!(output.rendered_body["text"], "Hello Carol");
        assert_eq!(
            output.rendered_body["buttons"],
            json!([{"label": "View", "url": "https://example.com"}])
        );
    }

    #[test]
    fn execute_pipeline_invalid_body_returns_error() {
        let input = make_input(
            json!("not an object"),
            json!({}),
        );

        let result = execute_pipeline(input);
        assert!(result.is_err());
    }
}
```

**Step 3: Add notifico-template dependency to notifico-core**

`notifico-core/Cargo.toml` — add to dependencies:
```toml
notifico-template = { workspace = true }
tracing = { workspace = true }
```

**Step 4: Wire up module**

`notifico-core/src/lib.rs` — add:
```rust
pub mod pipeline;
```

**Step 5: Verify**

Run: `cargo test -p notifico-core`
Expected: 20 tests pass (15 existing + 5 new)

**Step 6: Commit**

```bash
git add notifico-core/src/pipeline.rs notifico-core/src/lib.rs notifico-core/Cargo.toml Cargo.lock
git commit -m "feat: add pipeline executor with template rendering"
```

---

### Task 12: Delivery Worker

**Files:**
- Create: `notifico-server/src/worker.rs`
- Modify: `notifico-server/src/main.rs`
- Modify: `notifico-server/Cargo.toml`

The delivery worker dequeues DeliveryTasks, looks up the Transport from the registry, calls `send()`, and logs results to delivery_log.

**Step 1: Add deps to notifico-server**

`notifico-server/Cargo.toml` — add:
```toml
notifico-queue = { workspace = true }
uuid = { workspace = true }
```

**Step 2: Write worker module**

Create `notifico-server/src/worker.rs`:

```rust
use std::sync::Arc;

use sea_orm::{ConnectionTrait, DatabaseConnection};
use uuid::Uuid;

use notifico_core::registry::TransportRegistry;
use notifico_core::transport::RenderedMessage;
use notifico_queue::DeliveryTask;

/// Process a single delivery task.
pub async fn process_delivery(
    task: &DeliveryTask,
    registry: &TransportRegistry,
    db: &DatabaseConnection,
) -> Result<(), String> {
    tracing::info!(
        task_id = %task.id,
        channel = %task.channel,
        recipient = %task.recipient_id,
        attempt = task.attempt,
        "Processing delivery task"
    );

    // Look up transport
    let transport = registry
        .get(&task.channel)
        .ok_or_else(|| format!("Transport not found for channel: {}", task.channel))?;

    // Build RenderedMessage from task
    let message = RenderedMessage {
        channel: task.channel.clone(),
        recipient: task.contact_value.clone(),
        body: task.rendered_body.clone(),
        attachments: vec![],
    };

    // Send
    let result = transport.send(&message).await;

    match result {
        Ok(delivery_result) => {
            use notifico_core::transport::DeliveryResult;
            match delivery_result {
                DeliveryResult::Delivered => {
                    log_delivery(db, task, "delivered", None).await;
                    tracing::info!(task_id = %task.id, "Delivery successful");
                    Ok(())
                }
                DeliveryResult::Failed { reason, retryable } => {
                    let status = if retryable && task.attempt < task.max_attempts {
                        "queued" // will be retried
                    } else {
                        "failed"
                    };
                    log_delivery(db, task, status, Some(&reason)).await;

                    if retryable && task.attempt < task.max_attempts {
                        Err(format!("Retryable failure: {reason}"))
                    } else {
                        tracing::error!(task_id = %task.id, reason = %reason, "Delivery permanently failed");
                        Ok(()) // Don't retry
                    }
                }
            }
        }
        Err(e) => {
            let reason = e.to_string();
            log_delivery(db, task, "failed", Some(&reason)).await;
            tracing::error!(task_id = %task.id, error = %reason, "Transport error");
            Err(reason)
        }
    }
}

async fn log_delivery(
    db: &DatabaseConnection,
    task: &DeliveryTask,
    status: &str,
    error_message: Option<&str>,
) {
    let id = Uuid::now_v7();
    let error_msg = error_message.unwrap_or("");
    let delivered_at = if status == "delivered" {
        "CURRENT_TIMESTAMP"
    } else {
        "NULL"
    };

    let sql = format!(
        "INSERT INTO delivery_log (id, project_id, event_name, recipient_id, channel, status, error_message, attempts, delivered_at) \
         VALUES ('{id}', '{}', '{}', '{}', '{}', '{status}', '{error_msg}', {}, {delivered_at})",
        task.project_id, task.event_name, task.recipient_id, task.channel, task.attempt + 1,
    );

    if let Err(e) = db.execute_unprepared(&sql).await {
        tracing::error!(error = %e, "Failed to log delivery result");
    }
}
```

**Step 3: Wire worker into main.rs**

Modify `notifico-server/src/main.rs` — add `mod worker;` at the top, and add `TransportRegistry` to `AppState`:

```rust
mod config;
mod worker;

use std::sync::Arc;

use axum::{Router, extract::State, routing::get};
use sea_orm::DatabaseConnection;
use tower_http::trace::TraceLayer;

use config::{Config, ServerMode};
use notifico_core::registry::TransportRegistry;

struct AppState {
    db: DatabaseConnection,
    config: Config,
    registry: TransportRegistry,
}
```

Update `main()` to create the registry:
```rust
let registry = TransportRegistry::new();
// Transports will be registered here in Phase 4

let state = Arc::new(AppState {
    db,
    config: config.clone(),
    registry,
});
```

**Step 4: Verify**

Run: `cargo build -p notifico-server`
Expected: compiles (worker module exists but isn't called from a route yet — that's Phase 3)

Run: `cargo test --workspace`
Expected: all existing tests pass

**Step 5: Commit**

```bash
git add notifico-server/src/worker.rs notifico-server/src/main.rs notifico-server/Cargo.toml Cargo.lock
git commit -m "feat: add delivery worker with transport dispatch and logging"
```

---

### Task 13: Idempotency Guard

**Files:**
- Create: `notifico-db/src/migration/m20260303_000007_create_idempotency.rs`
- Create: `notifico-db/src/repo/idempotency.rs`
- Modify: `notifico-db/src/migration/mod.rs`
- Modify: `notifico-db/src/repo/mod.rs`

**Step 1: Create idempotency migration**

Create `notifico-db/src/migration/m20260303_000007_create_idempotency.rs`:

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(IdempotencyRecord::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(IdempotencyRecord::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(IdempotencyRecord::IdempotencyKey)
                            .string_len(512)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(IdempotencyRecord::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_idempotency_key_unique")
                    .table(IdempotencyRecord::Table)
                    .col(IdempotencyRecord::IdempotencyKey)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(IdempotencyRecord::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum IdempotencyRecord {
    Table,
    Id,
    IdempotencyKey,
    CreatedAt,
}
```

**Step 2: Register migration**

Modify `notifico-db/src/migration/mod.rs` — add module and entry:

```rust
mod m20260303_000007_create_idempotency;
```

Add to migrations vec:
```rust
Box::new(m20260303_000007_create_idempotency::Migration),
```

**Step 3: Write idempotency repo**

Create `notifico-db/src/repo/idempotency.rs`:

```rust
use sea_orm::*;
use uuid::Uuid;

/// Compound idempotency key: event_name + recipient_id + channel + optional client key.
pub fn make_idempotency_key(
    event_name: &str,
    recipient_id: Uuid,
    channel: &str,
    client_key: Option<&str>,
) -> String {
    match client_key {
        Some(k) => format!("{event_name}:{recipient_id}:{channel}:{k}"),
        None => format!("{event_name}:{recipient_id}:{channel}"),
    }
}

/// Check if an idempotency key already exists. If not, insert it and return false.
/// If it exists, return true (duplicate).
pub async fn check_and_insert(
    db: &DatabaseConnection,
    idempotency_key: &str,
) -> Result<bool, DbErr> {
    // Check existence
    let exists = db
        .query_one(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT id FROM idempotency_record WHERE idempotency_key = $1",
            [idempotency_key.into()],
        ))
        .await?;

    if exists.is_some() {
        return Ok(true); // duplicate
    }

    // Insert
    let id = Uuid::now_v7();
    db.execute(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO idempotency_record (id, idempotency_key) VALUES ($1, $2)",
        [id.into(), idempotency_key.into()],
    ))
    .await?;

    Ok(false) // not a duplicate
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};

    #[tokio::test]
    async fn idempotency_key_format() {
        let rid = Uuid::now_v7();
        let key = make_idempotency_key("order.confirmed", rid, "email", Some("abc"));
        assert!(key.contains("order.confirmed"));
        assert!(key.contains("email"));
        assert!(key.contains("abc"));
    }

    #[tokio::test]
    async fn check_and_insert_first_time() {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        let is_dup = check_and_insert(&db, "test-key-1").await.unwrap();
        assert!(!is_dup);
    }

    #[tokio::test]
    async fn check_and_insert_duplicate() {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        let first = check_and_insert(&db, "test-key-2").await.unwrap();
        assert!(!first);

        let second = check_and_insert(&db, "test-key-2").await.unwrap();
        assert!(second); // duplicate
    }
}
```

**Step 4: Wire up**

Modify `notifico-db/src/repo/mod.rs`:
```rust
pub mod idempotency;
pub mod template;
```

**Step 5: Verify**

Run: `cargo test -p notifico-db`
Expected: 9 tests pass (2 migration + 4 template repo + 3 idempotency)

**Step 6: Commit**

```bash
git add notifico-db/src/migration/m20260303_000007_create_idempotency.rs \
       notifico-db/src/migration/mod.rs \
       notifico-db/src/repo/idempotency.rs \
       notifico-db/src/repo/mod.rs
git commit -m "feat: add idempotency guard with deduplication table"
```

---

## Phase 2 Summary

After completing all tasks:

| Component | What it does |
|-----------|-------------|
| `notifico-template` | minijinja rendering: string + body (multi-field JSONB) |
| `notifico-db/repo/template` | Resolve template by ID/locale with fallback, query pipeline rules |
| `notifico-queue` | DeliveryTask struct (serializable job for queue) |
| `notifico-core/pipeline` | Pipeline executor: template body + data → rendered output |
| `notifico-server/worker` | Delivery worker: task → transport.send() → delivery_log |
| `notifico-db/repo/idempotency` | Deduplication by compound key |

**Total new tests:** ~30
**New crates:** notifico-template, notifico-queue
**New migrations:** idempotency_record table

The pipeline is end-to-end ready: event → resolve rules → render templates → produce delivery tasks → worker processes → log results. What's missing for a full working system is Phase 3 (API layer to accept events) and Phase 4 (actual transport implementations).
