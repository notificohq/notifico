# Phase 6: Admin CRUD API

## Goal
Add admin API endpoints for managing projects, events, pipeline rules,
templates, and credentials via HTTP. Uses existing `admin` scope auth.

## Depends on
- Phase 5 complete ✓

---

## Task 29: Admin repo CRUD functions

Add `notifico-db/src/repo/admin.rs` with insert/update/delete/list for
projects, events, rules, templates. 4 tests.

## Task 30: Admin HTTP handlers (projects, events, rules)

Add `notifico-server/src/admin.rs` with axum handlers. Mount under
`/admin/api/v1`. Auth requires admin scope. 2 integration tests.

## Task 31: Admin HTTP handlers (templates, credentials)

Add template and credential endpoints. 1 integration test.
