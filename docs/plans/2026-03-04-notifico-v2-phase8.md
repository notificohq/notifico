# Phase 8: Public API (Preferences & Unsubscribe)

## Goal
Add public-facing API endpoints that allow recipients to manage their
notification preferences and unsubscribe. Also wire preference checks
into the delivery pipeline so opted-out recipients are skipped.

## Depends on
- Phase 7 complete ✓

---

## Task 36: Preference and unsubscribe repo functions

Add `notifico-db/src/repo/preference.rs` with:
- list_preferences(recipient_id)
- set_preference(recipient_id, category, channel, enabled)
- is_opted_out(recipient_id, category, channel) → bool
- create_unsubscribe_token(recipient_id, event_id?, category?, channel?)
- find_by_unsubscribe_token(token)
- apply_unsubscribe(token)
3 tests.

## Task 37: Wire preference checks into pipeline

In ingest pipeline, after resolving rules and before enqueuing, check
recipient preferences. Skip delivery if opted out for that
category+channel. Log skipped deliveries.

## Task 38: Public API HTTP handlers

Add `notifico-server/src/public.rs` mounted at `/api/v1/public`:
- `GET /preferences?recipient={external_id}` — list preferences
- `PUT /preferences` — update a preference
- `POST /unsubscribe` — unsubscribe via token (no auth required)
- `GET /unsubscribe?token={token}` — one-click unsubscribe (GET for email links)
2 integration tests.
