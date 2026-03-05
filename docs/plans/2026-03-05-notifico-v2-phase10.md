# Phase 10: Webhook Transport, Graceful Shutdown & Health Checks

## Goal
Add a generic webhook transport for HTTP callbacks, improve the worker
with graceful shutdown, and make the /health endpoint verify DB
connectivity.

## Depends on
- Phase 9 complete ✓

---

## Task 42: Webhook (HTTP) transport

Add a generic webhook transport to notifico-core. Posts JSON payload to
a configurable URL via reqwest. Content schema: body (JSON, required),
method (text, optional, default POST). Credential schema: url (required),
headers (optional JSON object for auth headers), secret (optional HMAC
signing key). When secret is set, add `X-Notifico-Signature` header
with HMAC-SHA256 of the body.
3 tests.

## Task 43: Graceful worker shutdown

Wrap the worker loop in a tokio::select! that listens for SIGTERM/SIGINT
via tokio::signal. On signal, finish the current batch, then exit.
Log "Worker shutting down gracefully". No test (signal handling is
hard to unit-test), but verify compilation.

## Task 44: Health endpoint with DB check

Update /health to actually ping the DB (`SELECT 1`) and return
`{"status":"ok","db":"connected"}` on success or 503 with
`{"status":"degraded","db":"unreachable"}` on failure. 1 test.
