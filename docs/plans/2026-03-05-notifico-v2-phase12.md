# Phase 12: OpenAPI Spec, Template Preview & Event Stats

## Goal
Generate an OpenAPI spec from the existing routes, add a template preview
endpoint for testing templates without sending, and add delivery stats
per event for the admin dashboard.

## Depends on
- Phase 11 complete ✓

---

## Task 48: OpenAPI spec with utoipa

Add `utoipa` + `utoipa-axum` to workspace deps. Annotate all API
request/response types and handlers with `#[utoipa::path]` and
`ToSchema`. Serve the generated spec at `GET /api/openapi.json`.
1 test (spec is valid JSON with expected paths).

## Task 49: Template preview endpoint

Add `POST /admin/api/v1/templates/{id}/preview` — accepts
`{"locale": "en", "data": {...}}`, resolves the template, renders it
with the given data, and returns `{"rendered": {...}}` without
enqueuing any delivery. Useful for template authoring.
1 test.

## Task 50: Delivery stats per event

Add `GET /admin/api/v1/events/{id}/stats` — returns delivery counts
grouped by status (delivered, failed, queued, dead_letter) for the
event. Repo function `delivery_stats_by_event(db, event_id)`.
1 test.
