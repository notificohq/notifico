# Phase 4: Queue Integration & Console Transport

## Goal
Wire the ingest→queue→worker→transport pipeline end-to-end using a
database-backed task queue (no external dependencies like Redis needed for dev).
Add a `console` transport so the full flow can be exercised and tested.

## Architecture

```
POST /api/v1/events
  → ingest handler (Phase 3)
  → execute_pipeline → PipelineOutput
  → enqueue as delivery_task row (status=pending)     ← NEW
  → return task_ids

Worker loop (background)                               ← NEW
  → poll delivery_task WHERE status=pending
  → claim row (status=processing)
  → call process_delivery()
  → transport.send()
  → update status (completed / failed / dead_letter)
```

## Depends on
- Phase 3 complete (ingest, auth, recipient, pipeline) ✓

---

## Task 19: Add `delivery_task` migration

**Why:** Persist tasks in DB so workers can claim them across restarts.

### Steps

1. Create migration file `notifico-db/src/migration/m20260304_000008_create_delivery_task.rs`:

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
                    .table(DeliveryTask::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(DeliveryTask::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(DeliveryTask::ProjectId).uuid().not_null())
                    .col(ColumnDef::new(DeliveryTask::EventName).string_len(255).not_null())
                    .col(ColumnDef::new(DeliveryTask::RecipientId).uuid().not_null())
                    .col(ColumnDef::new(DeliveryTask::Channel).string_len(64).not_null())
                    .col(ColumnDef::new(DeliveryTask::ContactValue).string_len(512).not_null())
                    .col(ColumnDef::new(DeliveryTask::RenderedBody).json().not_null())
                    .col(ColumnDef::new(DeliveryTask::IdempotencyKey).string_len(512).null())
                    .col(
                        ColumnDef::new(DeliveryTask::Status)
                            .string_len(32)
                            .not_null()
                            .default("pending"),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::Attempt)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::MaxAttempts)
                            .integer()
                            .not_null()
                            .default(5),
                    )
                    .col(ColumnDef::new(DeliveryTask::ErrorMessage).text().null())
                    .col(
                        ColumnDef::new(DeliveryTask::NextRetryAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(DeliveryTask::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Index for worker polling: pending tasks ordered by next_retry_at
        manager
            .create_index(
                Index::create()
                    .name("idx_delivery_task_poll")
                    .table(DeliveryTask::Table)
                    .col(DeliveryTask::Status)
                    .col(DeliveryTask::NextRetryAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_delivery_task_project")
                    .table(DeliveryTask::Table)
                    .col(DeliveryTask::ProjectId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DeliveryTask::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum DeliveryTask {
    Table,
    Id,
    ProjectId,
    EventName,
    RecipientId,
    Channel,
    ContactValue,
    RenderedBody,
    IdempotencyKey,
    Status,
    Attempt,
    MaxAttempts,
    ErrorMessage,
    NextRetryAt,
    CreatedAt,
    UpdatedAt,
}
```

2. Register in `notifico-db/src/migration/mod.rs`:
   - Add `mod m20260304_000008_create_delivery_task;`
   - Add to `migrations()` vec

### Verify
```
cargo test -p notifico-db
```

---

## Task 20: Add queue repository (enqueue, claim, update)

**Why:** Repository layer for the task queue operations.

### Steps

1. Create `notifico-db/src/repo/queue.rs` with these functions:

```rust
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TaskRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub event_name: String,
    pub recipient_id: Uuid,
    pub channel: String,
    pub contact_value: String,
    pub rendered_body: Value,
    pub idempotency_key: Option<String>,
    pub status: String,
    pub attempt: i32,
    pub max_attempts: i32,
    pub error_message: Option<String>,
}

// Internal FromQueryResult struct with String UUIDs (SQLite compat)
#[derive(Debug, Clone, FromQueryResult)]
struct TaskRaw {
    id: String,
    project_id: String,
    event_name: String,
    recipient_id: String,
    channel: String,
    contact_value: String,
    rendered_body: String, // JSON stored as text in SQLite
    idempotency_key: Option<String>,
    status: String,
    attempt: i32,
    max_attempts: i32,
    error_message: Option<String>,
}

/// Insert a new delivery task with status='pending'.
pub async fn enqueue(
    db: &DatabaseConnection,
    id: Uuid,
    project_id: Uuid,
    event_name: &str,
    recipient_id: Uuid,
    channel: &str,
    contact_value: &str,
    rendered_body: &Value,
    idempotency_key: Option<&str>,
    max_attempts: i32,
) -> Result<(), DbErr>

/// Claim up to `limit` pending tasks (atomically set status='processing').
/// Returns claimed tasks.
pub async fn claim_pending(
    db: &DatabaseConnection,
    limit: u32,
) -> Result<Vec<TaskRow>, DbErr>

/// Mark task as completed.
pub async fn mark_completed(db: &DatabaseConnection, task_id: Uuid) -> Result<(), DbErr>

/// Mark task as failed. If retryable and attempts < max, set status back to 'pending'
/// with exponential backoff on next_retry_at. Otherwise set 'dead_letter'.
pub async fn mark_failed(
    db: &DatabaseConnection,
    task_id: Uuid,
    error: &str,
    retryable: bool,
    attempt: i32,
    max_attempts: i32,
) -> Result<(), DbErr>

/// Count tasks by status (for health checks / metrics).
pub async fn count_by_status(
    db: &DatabaseConnection,
) -> Result<Vec<(String, i64)>, DbErr>
```

Key implementation details:
- `claim_pending`: For SQLite, use a two-step approach:
  1. SELECT ids WHERE status='pending' AND next_retry_at <= CURRENT_TIMESTAMP LIMIT N
  2. UPDATE those ids SET status='processing', attempt=attempt+1
  This is safe for single-worker; for multi-worker Postgres, use `FOR UPDATE SKIP LOCKED`.
- `mark_failed` backoff: `next_retry_at = NOW() + 30s * 4^attempt` (30s, 2m, 8m, 32m)
- `rendered_body` stored as JSON text, parse back with `serde_json::from_str`

2. Register in `notifico-db/src/repo/mod.rs`: add `pub mod queue;`

### Tests (in queue.rs)
- `enqueue_and_claim`: enqueue 1 task, claim 1, verify fields
- `claim_skips_future_retry`: enqueue with future next_retry_at, claim returns empty
- `mark_completed_changes_status`: enqueue → claim → complete → re-claim returns empty
- `mark_failed_retryable_requeues`: fail with retryable=true, verify status goes back to pending
- `mark_failed_exhausted_goes_dead_letter`: fail when attempt >= max_attempts

### Verify
```
cargo test -p notifico-db
```

---

## Task 21: Wire ingest handler to enqueue tasks

**Why:** Replace the TODO — ingest should persist tasks in the queue table.

### Steps

1. In `notifico-server/src/ingest.rs`, after `execute_pipeline(pipeline_input)` succeeds:
   - Call `repo::queue::enqueue(...)` with the PipelineOutput fields
   - Keep pushing `output.id` to `task_ids`
   - If enqueue fails, push to `errors` instead

2. Remove the `// TODO: enqueue output as DeliveryTask` comment.

### Verify
```
cargo test -p notifico-server
```
The existing integration test `ingest_event_end_to_end` should still pass (it already
checks `accepted == 1` and `task_ids.len() == 1`). Bonus: add assertion that
delivery_task row exists in DB after ingest.

---

## Task 22: Add console transport

**Why:** A simple transport that logs messages to stdout. Needed to exercise
the full pipeline without external SMTP/SMS providers.

### Steps

1. Create `notifico-core/src/transport/console.rs` (refactor transport.rs into a module):
   - Move transport types to `notifico-core/src/transport/mod.rs`
   - Create `notifico-core/src/transport/console.rs`

Actually, simpler: just add `ConsoleTransport` to `notifico-core/src/transport.rs`:

```rust
/// A transport that logs messages to stdout. Useful for development and testing.
pub struct ConsoleTransport;

#[async_trait]
impl Transport for ConsoleTransport {
    fn channel_id(&self) -> ChannelId {
        ChannelId::new("console")
    }

    fn display_name(&self) -> &str {
        "Console (stdout)"
    }

    fn content_schema(&self) -> ContentSchema {
        ContentSchema {
            fields: vec![ContentField {
                name: "text".into(),
                field_type: ContentFieldType::Text,
                required: true,
                description: "Message text to print".into(),
            }],
        }
    }

    fn credential_schema(&self) -> CredentialSchema {
        CredentialSchema { fields: vec![] }
    }

    async fn send(&self, message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        let text = message.content.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("[no text field]");

        tracing::info!(
            channel = %message.channel,
            recipient = %message.recipient_contact,
            text = %text,
            "Console transport: delivering message"
        );

        Ok(DeliveryResult::Delivered {
            provider_message_id: None,
        })
    }
}
```

2. Add `tracing` dependency to `notifico-core/Cargo.toml`.

### Tests
- `console_transport_sends_ok`: send a message, verify `Delivered` result
- `console_transport_schema`: verify channel_id, display_name, content_schema

### Verify
```
cargo test -p notifico-core
```

---

## Task 23: Implement worker loop and wire to main.rs

**Why:** Background loop that polls the queue, claims tasks, and processes them.

### Steps

1. Update `notifico-server/src/worker.rs`:
   - Add `pub async fn run_worker_loop(state: Arc<AppState>)` function:

```rust
pub async fn run_worker_loop(state: Arc<AppState>) {
    let poll_interval = std::time::Duration::from_secs(2);

    tracing::info!("Worker loop started");

    loop {
        // Claim pending tasks
        let tasks = match notifico_db::repo::queue::claim_pending(&state.db, 10).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(error = %e, "Failed to claim tasks");
                tokio::time::sleep(poll_interval).await;
                continue;
            }
        };

        if tasks.is_empty() {
            tokio::time::sleep(poll_interval).await;
            continue;
        }

        tracing::info!(count = tasks.len(), "Claimed delivery tasks");

        for task_row in &tasks {
            // Convert TaskRow → DeliveryTask
            let delivery_task = to_delivery_task(task_row);

            match process_delivery(&delivery_task, &state.registry, &state.db).await {
                Ok(()) => {
                    if let Err(e) = notifico_db::repo::queue::mark_completed(
                        &state.db, task_row.id
                    ).await {
                        tracing::error!(task_id = %task_row.id, error = %e, "Failed to mark completed");
                    }
                }
                Err(reason) => {
                    let retryable = true; // process_delivery returns Err only for retryable
                    if let Err(e) = notifico_db::repo::queue::mark_failed(
                        &state.db,
                        task_row.id,
                        &reason,
                        retryable,
                        task_row.attempt,
                        task_row.max_attempts,
                    ).await {
                        tracing::error!(task_id = %task_row.id, error = %e, "Failed to mark failed");
                    }
                }
            }
        }
    }
}
```

2. Update `notifico-server/src/main.rs`:
   - Register `ConsoleTransport` in registry at startup
   - In `ServerMode::All`: spawn worker loop as background task alongside API server
   - In `ServerMode::Worker`: run worker loop (replace the placeholder ctrl_c wait)

```rust
// In main(), after creating registry:
use notifico_core::transport::ConsoleTransport;
registry.register(Arc::new(ConsoleTransport));

// In match:
ServerMode::All => {
    let worker_state = state.clone();
    tokio::spawn(async move {
        worker::run_worker_loop(worker_state).await;
    });
    start_api_server(state).await;
}
ServerMode::Worker => {
    worker::run_worker_loop(state).await;
}
```

### Verify
```
cargo test --workspace
```

All existing tests should pass. The integration test in main.rs will now actually
enqueue tasks in the DB (Task 21), and the worker would process them if running.

---

## Summary

| Task | Description | New tests |
|------|-------------|-----------|
| 19 | delivery_task migration | migration runs ✓ |
| 20 | Queue repo (enqueue/claim/update) | 5 tests |
| 21 | Wire ingest → enqueue | update existing integration test |
| 22 | Console transport | 2 tests |
| 23 | Worker loop + wiring | compile + existing tests |

After Phase 4, the full event→queue→worker→transport pipeline works end-to-end
with the console transport. Phase 5 would add real transports (email via SMTP/lettre).
