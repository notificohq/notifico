# Push Notification Transports + Transport Crate Extraction

**Date:** 2026-03-09
**Status:** Approved

## Summary

Add three push notification transports (FCM, APNs, Web Push) and extract all
existing transports from `notifico-core` into separate library crates under
`transports/`.

---

## Directory Structure

```
transports/
├── console/          # crate: notifico-transport-console
├── email/            # crate: notifico-transport-email
├── slack/            # crate: notifico-transport-slack
├── discord/          # crate: notifico-transport-discord
├── telegram/         # crate: notifico-transport-telegram
├── twilio-sms/       # crate: notifico-transport-twilio-sms
├── webhook/          # crate: notifico-transport-webhook
├── fcm/              # crate: notifico-transport-fcm       (new)
├── apns/             # crate: notifico-transport-apns      (new)
└── web-push/         # crate: notifico-transport-web-push  (new)
```

Each crate:
- Has its own `Cargo.toml`, depends on `notifico-core` for the `Transport` trait
- Exports a single public struct implementing `Transport`
- Owns its transport-specific dependencies
- Contains unit tests

`notifico-core` becomes lean — keeps `Transport` trait, `TransportRegistry`,
`ChannelId`, schemas, `DeliveryResult`. No transport implementations.

`notifico-server` depends on all transport crates and registers them.

---

## FCM (Firebase Cloud Messaging) v1

**Channel ID:** `push_fcm`
**Crate:** `fcm-v1`
**API:** FCM HTTP v1 (OAuth2 service account auth)

### Credential Schema

| Field | Type | Required | Secret |
|-------|------|----------|--------|
| `service_account_json` | string | yes | yes |

### Content Schema

| Field | Type | Required |
|-------|------|----------|
| `title` | string | yes |
| `body` | string | yes |
| `image_url` | string | no |
| `data` | object | no |
| `click_action` | string | no |

**Contact value:** FCM device registration token (string).

**Retryable errors:** 5xx, 429. Not retryable: 404 (invalid token).

---

## APNs (Apple Push Notification service)

**Channel ID:** `push_apns`
**Crate:** `a2`
**API:** HTTP/2 provider API with token-based .p8 auth

### Credential Schema

| Field | Type | Required | Secret |
|-------|------|----------|--------|
| `team_id` | string | yes | no |
| `key_id` | string | yes | no |
| `private_key` | string | yes | yes |
| `environment` | string | yes | no |

`environment` is `"production"` or `"sandbox"`.

### Content Schema

| Field | Type | Required |
|-------|------|----------|
| `title` | string | yes |
| `body` | string | yes |
| `badge` | integer | no |
| `sound` | string | no |
| `data` | object | no |
| `category` | string | no |

**Contact value:** APNs device token (hex string).

**Retryable errors:** 5xx, `TooManyRequests`. Not retryable: `BadDeviceToken`, `Unregistered`.

---

## Web Push (RFC 8030 + VAPID)

**Channel ID:** `push_web`
**Crate:** `web-push`
**API:** RFC 8030 push endpoint with VAPID (RFC 8292) and encrypted payload (RFC 8291)

### Credential Schema

| Field | Type | Required | Secret |
|-------|------|----------|--------|
| `vapid_private_key` | string (base64url) | yes | yes |
| `vapid_public_key` | string (base64url) | yes | no |
| `subject` | string (mailto: or https://) | yes | no |

### Content Schema

| Field | Type | Required |
|-------|------|----------|
| `title` | string | yes |
| `body` | string | yes |
| `icon` | string | no |
| `url` | string | no |
| `badge` | string | no |
| `data` | object | no |

**Contact value:** Push subscription JSON (`{"endpoint":"...","keys":{"p256dh":"...","auth":"..."}}`).

**Retryable errors:** 5xx, 429. Not retryable: 410 (Gone, subscription expired).

---

## Common Patterns

- All transports implement the existing `Transport` trait with no changes
- Invalid/expired tokens return `DeliveryResult { retryable: false }` to avoid
  wasting retry attempts
- Each transport is registered in `main.rs` alongside existing transports
