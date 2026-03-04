# Phase 9: Deployment & Observability

## Goal
Make Notifico deployable with Docker and add Prometheus metrics for
production monitoring.

## Depends on
- Phase 8 complete ✓

---

## Task 39: Dockerfile and docker-compose

Create multi-stage Dockerfile (builder + runtime). docker-compose.yml
with notifico + PostgreSQL for quick local setup. Include .dockerignore.

## Task 40: Prometheus /metrics endpoint

Add `GET /metrics` endpoint exposing Prometheus-format metrics:
- HTTP request count/duration (by method, path, status)
- Delivery task counts (by status)
- Queue depth
Use `metrics` + `metrics-exporter-prometheus` crates.
1 test.

## Task 41: Telegram transport

Add Telegram Bot API transport to notifico-core. Sends messages via
HTTP to `api.telegram.org`. Content schema: text (required),
parse_mode (optional). Credential schema: bot_token.
2 tests.
