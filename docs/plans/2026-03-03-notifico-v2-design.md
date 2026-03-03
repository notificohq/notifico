# Notifico v2 — Design Document

**Date:** 2026-03-03
**Status:** Approved
**Scope:** Full rewrite from scratch

## Problem Statement

Every backend project needs a user notification service. The same requirements repeat:
editable templates without developer involvement, versioning, multi-channel delivery
(email, SMS, push, messengers), user preference management, mass broadcasts, reliable
delivery guarantees, and unsubscribe support.

Notifico v2 is a self-hosted notification server that solves this universally.

---

## Decisions Summary

| Aspect | Decision |
|--------|----------|
| Architecture | Hybrid: monolith by default, `--mode api/worker/all` |
| Backend | Rust, axum, sea-orm |
| Frontend | React + Shadcn + TanStack + Tailwind, GrapesJS + MJML for email |
| DB | PostgreSQL 18 + SQLite |
| Queue | Apalis (apalis-redis, apalis-postgres, apalis-sqlite, apalis-amqp) |
| Cache | Valkey 9 |
| Templates | minijinja (Jinja2), versioning, multilingual |
| Channels | Registry-based `Transport` trait, extensible (WASM/WASI future) |
| In-App Inbox | First-class channel, WebSocket real-time, headless JS SDK + React UI kit |
| v1 Transports | email, sms (Twilio), push (FCM, APNs, Web Push), telegram, max, discord, slack, **inbox** |
| API | REST + OpenAPI (utoipa) -> autogen TypeScript client |
| Auth | API keys + JWT + OIDC optional |
| User preferences | Public API + admin panel |
| Observability | Prometheus `/metrics` + OpenTelemetry (metrics + traces) |
| Deploy | Docker, self-hosted, prepared for SaaS |
| Differentiation | Lightweight single-binary, Telegram full + MAX, template versioning + multilingual |
| Roadmap | OTP/Magic Link module, MySQL support, WASM/WASI plugins, WebTransport |

---

## 1. Core — Event Pipeline

### Data Flow

```
Client App                    Notifico
    |                            |
    +-- POST /api/v1/events --> Ingest API
    |   {                        |
    |     "event": "order.confirmed",
    |     "recipients": [        +-- Validate + enrich
    |       {"id": "user-123",   |
    |        "email": "...",     +-- Check user preferences
    |        "phone": "..."}     |   (channel enabled? schedule OK?)
    |     ],                     |
    |     "data": {              +-- Route to channels
    |       "order_id": 42,      |   (email, sms, push, telegram...)
    |       "total": "99.90"     |
    |     }                      +-- Render templates (minijinja)
    |   }                        |   per channel, per locale
    |                            |
    |                            +-- Enqueue delivery tasks
    |                            |   (Valkey / AMQP / DB)
    |                            |
    |                            +-- Workers deliver
    |                                +-- SMTP -> email
    |                                +-- Twilio -> SMS
    |                                +-- FCM -> push
    |                                +-- Bot API -> messenger
```

### Key Entities

- **Event** — named event (`order.confirmed`, `user.signup`). Defined in admin panel.
- **Channel** — delivery method. String identifier, not enum. Registry-based.
- **Pipeline Rule** — binding: event -> channel + template + conditions.
- **Template** — minijinja template with versioning and multilingual support.
- **Recipient** — external user ID + contact info. Notifico does not store user passwords.

### Notification Categories

Each event has a category:
- **transactional** — cannot unsubscribe (order confirmation, password reset)
- **marketing** — can unsubscribe, respects user schedules
- **system** — technical notifications (monitoring), no user preferences

### Multilingual Support

- Recipient has `locale` field (e.g., `"ru"`, `"en"`, `"de"`)
- Template stores content per locale per version
- Rendering: pick template content by `recipient.locale`, fallback to project default locale

```
Template "order_confirmed_email" (channel: email)
+-- v3 (current)
|   +-- ru: { subject, html_body, text_body }
|   +-- en: { subject, html_body, text_body }
|   +-- de: { subject, html_body, text_body }
+-- v2 (previous)
    +-- ru: ...
    +-- en: ...
```

---

## 2. Data Model

### Projects (multi-tenancy ready)

```
project
+-- id: UUID
+-- name: String
+-- default_locale: String
+-- settings: JSONB (rate limits, retry policy, etc.)
+-- created_at / updated_at
```

Every entity below is scoped to `project_id`. Currently one instance = one project,
but isolation is built in for future SaaS.

### Events & Pipeline Rules

```
event
+-- id: UUID
+-- project_id: UUID
+-- name: String (unique per project)
+-- category: Enum (transactional | marketing | system)
+-- description: String

pipeline_rule
+-- id: UUID
+-- event_id: UUID
+-- channel: String (extensible, not enum)
+-- template_id: UUID
+-- enabled: bool
+-- conditions: JSONB (optional)
+-- priority: i32
```

### Templates

```
template
+-- id: UUID
+-- project_id: UUID
+-- name: String (unique per project)
+-- channel: String
+-- created_at / updated_at

template_version
+-- id: UUID
+-- template_id: UUID
+-- version: i32
+-- is_current: bool
+-- created_at

template_content
+-- id: UUID
+-- template_version_id: UUID
+-- locale: String
+-- body: JSONB (schema defined by channel's content_schema)
+-- updated_at
```

### Recipients & Contacts

```
recipient
+-- id: UUID
+-- project_id: UUID
+-- external_id: String (unique per project)
+-- locale: String
+-- timezone: String
+-- metadata: JSONB
+-- created_at / updated_at

recipient_contact
+-- id: UUID
+-- recipient_id: UUID
+-- channel: String
+-- value: String
+-- verified: bool
+-- created_at
```

### User Preferences

```
recipient_preference
+-- id: UUID
+-- recipient_id: UUID
+-- category: Enum (marketing | transactional | system)
+-- channel: String
+-- enabled: bool
+-- schedule_start: Time?
+-- schedule_end: Time?
+-- updated_at
```

### Unsubscribe

```
unsubscribe
+-- id: UUID
+-- recipient_id: UUID
+-- event_id: UUID? (null = all events of category)
+-- category: Enum?
+-- channel: String?
+-- token: String (unique, for List-Unsubscribe URL)
+-- created_at
```

### Delivery Log

```
delivery_log
+-- id: UUID
+-- project_id: UUID
+-- event_name: String
+-- recipient_id: UUID
+-- channel: String
+-- status: Enum (queued | sending | delivered | failed | bounced)
+-- error_message: String?
+-- attempts: i32
+-- created_at
+-- delivered_at?
```

---

## 3. API Design

### Three API groups

**Ingest API** (authn: API key, scope: `ingest`)

```
POST /api/v1/events              -- Send event
POST /api/v1/events/batch        -- Batch send (mass broadcasts)
```

**Public API** (authn: API key, scope: `public`)

```
GET  /api/v1/recipients/{id}/preferences
PUT  /api/v1/recipients/{id}/preferences
POST /api/v1/unsubscribe/{token}
GET  /api/v1/unsubscribe/{token}          -- List-Unsubscribe (RFC 8058)
POST /api/v1/recipients/{id}/contacts
DELETE /api/v1/recipients/{id}/contacts/{cid}
```

**Admin API** (authn: session/JWT + OIDC optional)

```
GET/POST/PUT/DELETE  /admin/api/v1/projects
GET/POST/PUT/DELETE  /admin/api/v1/events
GET/POST/PUT/DELETE  /admin/api/v1/events/{id}/rules
GET/POST/PUT/DELETE  /admin/api/v1/templates
GET/POST             /admin/api/v1/templates/{id}/versions
GET/PUT              /admin/api/v1/templates/{id}/versions/{v}/content/{locale}
POST                 /admin/api/v1/templates/{id}/preview
GET/POST/PUT         /admin/api/v1/recipients
GET/PUT              /admin/api/v1/recipients/{id}/preferences
GET/POST             /admin/api/v1/broadcasts
GET                  /admin/api/v1/broadcasts/{id}/status
GET                  /admin/api/v1/channels
GET                  /admin/api/v1/channels/{id}/schema
GET/POST/PUT/DELETE  /admin/api/v1/credentials
GET                  /admin/api/v1/delivery-log
GET/POST/DELETE      /admin/api/v1/api-keys
```

All APIs are REST + OpenAPI. TypeScript client autogenerated via `openapi-typescript`.

---

## 4. Queue & Delivery

### Architecture

```
Ingest API --> Queue --> Workers
    |                       |
    +-- validate            +-- pick task
    +-- check preferences   +-- send via Transport
    +-- resolve channels    +-- success -> log delivered
    +-- render templates    +-- fail -> retry with backoff
    +-- enqueue tasks            +-- max retries -> log failed
```

### Queue Backends

Configured via `[queue]` section. Uses existing apalis crates:
- `apalis-redis` (Valkey) — default
- `apalis-postgres`
- `apalis-sqlite`
- `apalis-amqp` — for HA/scaling

### Retry & Backoff

- Exponential backoff: 30s -> 2m -> 8m -> 30m (configurable)
- Max retries per channel (default: 5)
- Dead letter queue for permanently failed
- Separate retry policy for transactional (more retries) vs marketing

### Mass Broadcasts

```json
{
  "name": "Black Friday",
  "event": "promo.black_friday",
  "filter": { "locale": "ru", "metadata.city": "Moscow" },
  "scheduled_at": "2026-03-15T10:00:00Z"
}
```

- Batch processing: 1000 recipients per batch
- Rate limiting per channel (configurable)
- Status: draft -> scheduled -> processing -> completed / partially_failed
- Cancellable during processing

### Delivery Guarantees

- **At-least-once**: task ACK only after successful send
- **Idempotency**: deduplication by (event_name, recipient_id, channel, idempotency_key)
- **Outbox pattern** for critical transactional: write to DB first, then queue

---

## 5. Email Transport

### Rendering Pipeline

```
GrapesJS editor -> MJML source + editor_json (stored in template_content)
                          |
                   minijinja renders MJML with data
                          |
                   mrml (Rust MJML parser) -> HTML
                          |
                   html2text -> text fallback
                          |
                   lettre assembles MIME:
                     - multipart/alternative: text + html
                     - attachments (multipart/mixed)
                     - inline images (Content-ID)
                     - List-Unsubscribe header
```

### Key Dependencies

| Task | Crate |
|------|-------|
| SMTP sending | `lettre` |
| MJML -> HTML | `mrml` |
| HTML -> text | `html2text` |

### List-Unsubscribe (RFC 8058)

Every marketing email includes:
```
List-Unsubscribe: <https://notifico.example.com/api/v1/unsubscribe/{token}>
List-Unsubscribe-Post: List-Unsubscribe=One-Click
```

### Attachments

Stored in asset storage (filesystem / S3-compatible). Referenced by asset ID in templates.
Support for inline images via Content-ID (`<img src="cid:logo123">`).

---

## 6. Transports

All transports implement `Transport` trait. Each is a separate crate.

### Transport Trait (extensible)

```rust
type ChannelId = String;

#[async_trait]
trait Transport: Send + Sync {
    fn channel_id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn content_schema(&self) -> ContentSchema;
    fn credential_schema(&self) -> CredentialSchema;
    async fn send(&self, message: RenderedMessage) -> Result<DeliveryResult>;
}
```

Channels are String-based (not enum) for extensibility. Future: WASM/WASI plugins.

### v1 Transports

| Crate | Channel ID | Protocol | Content Schema |
|-------|-----------|----------|----------------|
| `notifico-smtp` | `email` | SMTP + MJML | subject, mjml, editor_json |
| `notifico-sms-twilio` | `sms` | Twilio REST API | text |
| `notifico-push-fcm` | `push_fcm` | FCM HTTP v1 | title, body, image_url, data |
| `notifico-push-apns` | `push_apns` | APNs HTTP/2 | title, body, badge, sound, data |
| `notifico-push-web` | `push_web` | Web Push RFC 8030 + VAPID | title, body, icon, url |
| `notifico-telegram` | `telegram` | Telegram Bot API | text, parse_mode |
| `notifico-max` | `max` | VK MAX Bot API | text |
| `notifico-discord` | `discord` | Discord Webhook/Bot API | text, embeds |
| `notifico-slack` | `slack` | Slack Web API/Webhook | text, blocks |
| `notifico-inbox` | `inbox` | Local DB + WebSocket | title, body (md), redirect_url, tags, actions, data |

### Credentials

Stored encrypted in DB (AES-256-GCM). Each transport declares required credentials
via `credential_schema()`.

### SMS Extensibility

Additional SMS providers (MessageBird, Vonage) added as separate crates.
Pipeline rule specifies credential which determines provider.

---

## 7. Frontend & Admin Panel

### Stack

- React 18+ with TypeScript
- Shadcn/ui components
- TanStack Query (data fetching) + TanStack Router (routing)
- Tailwind CSS
- GrapesJS + MJML preset (email block editor)
- Refine.js (evaluate fit with Shadcn + TanStack, use if suitable)

### OpenAPI -> TypeScript

```
Rust (utoipa) -> openapi.json -> openapi-typescript -> typed client
```

### Admin Screens

- Dashboard (delivery stats, charts, errors)
- Events (CRUD + pipeline rules)
- Templates (list, editor per channel, version history)
  - Email: GrapesJS block editor
  - SMS/Push/Messenger: text editor with Jinja2
- Recipients (list, search, contacts, preferences)
- Broadcasts (create, manage, status)
- Delivery Log (filterable, searchable)
- Channels (registered channels + credentials)
- API Keys
- Settings (project, default locale, rate limits)

### Embedding

Frontend built to static assets, embedded in Rust binary via `rust-embed` or
`include_dir`. Single binary serves API + SPA. Dev mode: proxy to Vite dev server.

---

## 8. Auth & Security

### Three Authentication Levels

**API Keys** (Ingest + Public API):
- Prefix: `nk_live_` (production), `nk_test_` (sandbox)
- Hashed in DB (SHA-256), shown once on creation
- Scopes: `ingest`, `public`, `admin`
- Rate limiting per key

**Session/JWT** (Admin panel):
- Local users: email + password (argon2)
- JWT access token (15 min) + refresh token (7 days)
- CSRF protection

**OIDC** (optional SSO for admin):
- Configurable via `[auth.oidc]`
- Can disable local users when OIDC is active

### Encryption at Rest

- Transport credentials encrypted AES-256-GCM
- Master key from env: `NOTIFICO_ENCRYPTION_KEY`
- Key rotation supported

### Multi-tenancy Isolation

- Every request scoped to `project_id`
- API key -> project_id lookup
- All SQL queries filtered by `project_id` (sea-orm global filter)

### Rate Limiting

- Per API key: configurable (default 1000 req/min)
- Per channel delivery: configurable per transport
- Implementation via Valkey (sliding window counter)

---

## 9. Configuration & Deployment

### Configuration

Single `notifico.toml` + env overrides (`NOTIFICO_SERVER_PORT=9000`):

```toml
[server]
mode = "all"  # all | api | worker
host = "0.0.0.0"
port = 8000
admin_port = 8001

[database]
backend = "postgres"  # postgres | sqlite
url = "postgres://user:pass@localhost/notifico"

[queue]
backend = "redis"  # redis | postgres | sqlite | amqp

[storage]
backend = "filesystem"  # filesystem | s3

[auth]
encryption_key = "${NOTIFICO_ENCRYPTION_KEY}"
jwt_secret = "${NOTIFICO_JWT_SECRET}"

[auth.oidc]
enabled = false
```

### Docker

Single multi-stage image. One binary serves API + frontend.

### docker-compose (simple)

```yaml
services:
  notifico:
    image: notifico:latest
    ports: ["8000:8000", "8001:8001"]
    depends_on: [postgres, valkey]
  postgres:
    image: postgres:18
  valkey:
    image: valkey/valkey:9
```

### docker-compose (HA)

```yaml
services:
  notifico-api:
    image: notifico:latest
    command: ["notifico", "--mode", "api"]
    deploy: { replicas: 2 }
  notifico-worker:
    image: notifico:latest
    command: ["notifico", "--mode", "worker"]
    deploy: { replicas: 4 }
  postgres:
    image: postgres:18
  valkey:
    image: valkey/valkey:9
```

### Observability

- **Prometheus**: `/metrics` endpoint for scraping
- **OpenTelemetry**: metrics + traces (tracing-opentelemetry + OTLP exporter)
- **Health**: `GET /health` (liveness), `GET /ready` (readiness)
- **Delivery log**: queryable via Admin API + UI

---

## 10. Crate Structure

```
notifico/
+-- notifico-server/          # Axum HTTP server, routing, middleware
+-- notifico-core/            # Pipeline engine, event processing, Transport trait
+-- notifico-queue/           # Queue abstraction (apalis backends)
+-- notifico-db/              # sea-orm models, migrations, repositories
+-- notifico-template/        # minijinja templating + versioning
+-- notifico-preferences/     # User preference engine
+-- notifico-broadcast/       # Mass broadcast processing
+-- notifico-attachment/      # Asset/attachment storage
+-- transports/
|   +-- notifico-smtp/
|   +-- notifico-sms-twilio/
|   +-- notifico-push-fcm/
|   +-- notifico-push-apns/
|   +-- notifico-push-web/
|   +-- notifico-telegram/
|   +-- notifico-max/
|   +-- notifico-discord/
|   +-- notifico-slack/
|   +-- notifico-inbox/       # In-app inbox transport + WS + REST
+-- sdks/
|   +-- js/                   # @notifico/js (headless, framework-agnostic)
|   +-- react/                # @notifico/react (UI kit)
+-- notifico-admin-api/       # Admin REST API
+-- notifico-public-api/      # Public REST API
+-- notifico-ingest-api/      # Ingest REST API
+-- notificox/                # CLI companion tool
+-- frontend/                 # React SPA
```

---

## Differentiation & In-App Inbox

See `2026-03-03-notifico-v2-differentiation-inbox.md` for full design:
- Lightweight single-binary (SQLite for dev, PG for prod, vs Novu's MongoDB + 2 Redis + Node.js)
- Russian-language messengers (Telegram full feature set + VK MAX)
- Template versioning + multilingual (no competitor has both natively)
- In-App Inbox as first-class channel with WebSocket real-time, headless JS SDK, React UI kit

---

## Roadmap (post-v1)

- **notifico-otp** — OTP codes and magic link module
- **MySQL support** — via sea-orm + apalis-mysql
- **WASM/WASI plugin system** — custom transports as WASM modules
- **WebTransport** — alternative to WebSocket for inbox real-time
- **WhatsApp Business** transport
- **Additional SMS providers** (MessageBird, Vonage)
- **Vue/Svelte SDK** — UI kits beyond React
- **Mobile SDKs** — React Native, Flutter wrappers
- **Digest/batching** — combine multiple inbox notifications
