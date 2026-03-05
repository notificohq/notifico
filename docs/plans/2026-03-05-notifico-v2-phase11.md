# Phase 11: Broadcasts, Rate Limiting & JSON Logging

## Goal
Add broadcast (batch send) capability, per-API-key rate limiting on the
ingest endpoint, and structured JSON logging for production.

## Depends on
- Phase 10 complete ✓

---

## Task 45: Broadcast endpoint

Add `POST /api/v1/broadcasts` — accepts an event name, optional recipient
filter (project-wide or by external_id list), and template data. Resolves
all matching recipients, runs the pipeline for each, and enqueues tasks.
Returns `{"broadcast_id": uuid, "recipient_count": N}`.
Repo function: `list_recipients_by_project(db, project_id)` to fetch all
recipients for a project.
2 tests (unit + integration).

## Task 46: API key rate limiting

Add in-memory sliding-window rate limiting keyed by API key ID. Default
100 req/min, configurable per key via `rate_limit` column (already in
api_key table). Return 429 with Retry-After header when exceeded.
Use a simple `DashMap<Uuid, Vec<Instant>>` — no external deps.
2 tests.

## Task 47: Structured JSON logging

Add `--log-format json` CLI flag (or `NOTIFICO_LOG_FORMAT=json` env var).
When set, use `tracing_subscriber::fmt::json()` instead of the default
text format. Wire into config and main startup.
1 test (config parsing).
