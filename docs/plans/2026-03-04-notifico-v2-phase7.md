# Phase 7: Extended Admin API (Recipients, Delivery Log, API Keys, Channels)

## Goal
Complete the admin API surface: recipient management, delivery log
querying, API key CRUD, and channels list with schema info.

## Depends on
- Phase 6 complete ✓

---

## Task 32: Recipient admin repo functions

Add to `notifico-db/src/repo/admin.rs`: list_recipients, get_recipient
(with contacts), create/update/delete recipient, list/add/delete contacts.
3 tests.

## Task 33: Delivery log repo + write-on-complete

Add `notifico-db/src/repo/delivery_log.rs` with insert_log and
list_logs (filter by project, optional status/event/recipient filters,
pagination). Wire worker to write delivery_log on task completion/failure.
2 tests.

## Task 34: API key admin repo functions

Add to `notifico-db/src/repo/api_key.rs`: list_keys, create_key
(returns raw key once), delete_key, toggle_enabled. 2 tests.

## Task 35: Admin HTTP handlers (recipients, delivery log, API keys, channels)

Add endpoints to admin router:
- `/recipients` GET, POST; `/recipients/{id}` GET, PUT, DELETE
- `/recipients/{id}/contacts` GET, POST; `/contacts/{id}` DELETE
- `/delivery-log` GET (with query params)
- `/api-keys` GET, POST; `/api-keys/{id}` DELETE
- `/channels` GET (returns registered transports with schemas)
2 integration tests.
