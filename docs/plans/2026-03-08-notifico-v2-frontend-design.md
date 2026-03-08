# Notifico v2 Frontend & Middleware Design

## Overview

Admin panel SPA for Notifico, providing full CRUD management of all backend resources.
Pipeline middleware system for intercepting and transforming messages at various stages.

## Stack

- **Svelte 5** + **SvelteKit** (adapter-static for SPA output)
- **TypeScript** throughout
- **shadcn-svelte** for UI components
- **TanStack Query** (Svelte) for data fetching/caching
- **Tailwind CSS** for styling
- **Bun** as package manager and script runner

## Deployment

- Built as static SPA via `@sveltejs/adapter-static`
- Embedded in the Rust binary via `rust-embed`
- Served by axum — static files with SPA fallback to `index.html`
- API routes (`/api/`, `/admin/api/`, `/health`, `/metrics`) take precedence over SPA fallback
- Single binary deployment

## Authentication

- Simple API key login page — user enters an admin-scoped API key
- Stored in memory with optional localStorage persistence
- Sent as `Authorization: Bearer <key>` on all API requests
- Auth guard on layout redirects to `/login` if no key present
- JWT/OIDC deferred to a future phase

## Project Structure

```
notifico-frontend/
├── src/
│   ├── lib/
│   │   ├── api/          # Typed API client functions
│   │   ├── components/   # Shared UI components
│   │   └── stores/       # Auth store, app state
│   ├── routes/
│   │   ├── +layout.svelte       # App shell: sidebar nav, auth guard
│   │   ├── login/+page.svelte
│   │   ├── dashboard/+page.svelte
│   │   ├── events/
│   │   │   ├── +page.svelte          # Event list
│   │   │   └── [id]/+page.svelte     # Event detail + pipeline rules
│   │   ├── templates/
│   │   │   ├── +page.svelte          # Template list
│   │   │   └── [id]/+page.svelte     # Template content editor per locale
│   │   ├── recipients/
│   │   │   ├── +page.svelte          # Recipient list
│   │   │   └── [id]/+page.svelte     # Recipient detail: contacts, preferences
│   │   ├── credentials/+page.svelte
│   │   ├── delivery-log/+page.svelte
│   │   ├── broadcasts/+page.svelte
│   │   ├── api-keys/+page.svelte
│   │   └── settings/+page.svelte
│   ├── app.html
│   ├── app.css
│   └── app.d.ts
├── static/
├── svelte.config.js
├── tailwind.config.ts
├── vite.config.ts
├── tsconfig.json
├── package.json
└── bunfig.toml
```

## Pages

| Page | Description | API Endpoints |
|------|-------------|---------------|
| Login | API key entry form | Validates with `GET /admin/api/v1/projects` |
| Dashboard | Delivery stats, recent activity | `GET /events/{id}/stats`, `GET /delivery-log` |
| Events | List + CRUD, pipeline rules per event | `GET/POST/PUT/DELETE /events`, `/events/{id}/rules` |
| Templates | List + CRUD, content editor per locale, preview | `GET/POST/DELETE /templates`, `PUT /content/{locale}`, `POST /preview` |
| Recipients | List + CRUD, contacts, preferences | `GET/POST/PUT/DELETE /recipients`, contacts, preferences |
| Credentials | Manage transport credentials | `GET/POST/DELETE /credentials` |
| Delivery Log | Searchable/filterable table with pagination | `GET /delivery-log` |
| Broadcasts | Send broadcast form | `POST /broadcasts` |
| API Keys | List + create/delete | `GET/POST/DELETE /api-keys` |
| Settings | Project settings, channel overview | `GET /projects`, `GET /channels` |

## API Client

- Hand-written typed API client using `fetch`
- Base URL configurable (defaults to same origin)
- Centralized error handling (401 → redirect to login, 4xx/5xx → toast)
- TanStack Query for caching, refetching, optimistic updates

## Rust Embedding

- New `notifico-frontend` crate (or module in notifico-server) using `rust-embed`
- Points to `notifico-frontend/build/` (SvelteKit adapter-static output)
- Axum fallback route serves embedded files
- SPA fallback: any non-API, non-file path returns `index.html`
- Build pipeline: `bun install && bun run build` before `cargo build --release`

## UI Layout

- Sidebar navigation (collapsible) with page links
- Top bar with project name and logout
- Content area with breadcrumbs
- Responsive: sidebar collapses on mobile
- Dark/light mode via Tailwind (system preference default)

---

## Pipeline Middleware System

### Overview

Middleware intercepts messages at defined hook points in the pipeline. Implementations
are native Rust traits; activation and configuration per-event stored in DB and managed
via admin UI. The trait is designed so that WASM-based middleware can be added in the
future without changing the interface.

### Hook Points

| Hook | When | Use Cases |
|------|------|-----------|
| **pre-render** | Before template rendering | Modify context data, inject variables, filter recipients |
| **post-render** | After rendering, before enqueue | Modify content — tracking pixels, unsubscribe links, URL rewriting |
| **pre-send** | After dequeue, before transport.send() | Last-chance modifications, per-recipient rate limiting |
| **post-send** | After transport.send() | Logging, webhooks, analytics |

All four hooks defined in the trait with default no-op implementations.
Initial implementation covers **post-render** and **post-send** only.

### Middleware Trait

```rust
#[async_trait]
pub trait Middleware: Send + Sync {
    fn name(&self) -> &str;
    fn hook_point(&self) -> HookPoint;

    async fn pre_render(&self, input: &mut PipelineInput, config: &Value) -> Result<(), CoreError> { Ok(()) }
    async fn post_render(&self, output: &mut PipelineOutput, config: &Value) -> Result<(), CoreError> { Ok(()) }
    async fn pre_send(&self, message: &mut RenderedMessage, config: &Value) -> Result<(), CoreError> { Ok(()) }
    async fn post_send(&self, message: &RenderedMessage, result: &DeliveryResult, config: &Value) -> Result<(), CoreError> { Ok(()) }
}
```

### Configuration Model

Middleware is attached per pipeline rule (event + channel). Stored in DB:

```
pipeline_middleware (
    id UUID,
    rule_id UUID FK,        -- references pipeline_rule
    middleware_name TEXT,    -- e.g. "unsubscribe_link"
    config JSONB,           -- middleware-specific config
    priority INT,           -- execution order (lower = first)
    enabled BOOL
)
```

Admin API endpoints:
- `GET /admin/api/v1/rules/{rule_id}/middleware`
- `POST /admin/api/v1/rules/{rule_id}/middleware`
- `PUT /admin/api/v1/middleware/{id}`
- `DELETE /admin/api/v1/middleware/{id}`

### Day-1 Middleware

| Name | Hook | Description |
|------|------|-------------|
| `unsubscribe_link` | post-render | Inject List-Unsubscribe header (RFC 8058) and unsubscribe URL into email body |
| `click_tracking` | post-render | Rewrite URLs to route through Notifico (`/t/click/{token}`) for click analytics |
| `open_tracking` | post-render | Append invisible 1x1 pixel (`/t/open/{token}`) to HTML emails for open tracking |
| `utm_params` | post-render | Auto-append UTM tags to links (source, medium, campaign from event name) |
| `plaintext_fallback` | post-render | Auto-generate text/plain from HTML content via html2text |

### Tracking Endpoints

New public routes for tracking pixel/click resolution:

- `GET /t/open/{token}` — Returns 1x1 transparent GIF, records open event
- `GET /t/click/{token}` — Records click event, 302 redirects to original URL

Token encodes: delivery_log_id + link_url (for clicks). Stored/resolved via DB or signed JWT.

### Future: WASM Middleware

The `Middleware` trait is designed so WASM modules can implement it:
- WASM module exports the same hook functions
- A `WasmMiddleware` adapter wraps wasmtime/wasmer calls behind the trait
- WASM modules uploaded via admin API, stored in DB or filesystem
- Sandboxed execution with resource limits

### OpenTelemetry

- Add `opentelemetry` + `tracing-opentelemetry` crates
- Instrument middleware chain: each middleware call is a span with name, hook point, duration
- Instrument transport.send(): span per delivery with channel, recipient, status
- Instrument pipeline execution: parent span covering render → middleware → enqueue
- Configurable OTLP exporter endpoint via `NOTIFICO_OTEL_ENDPOINT` env var
- Graceful degradation: if no endpoint configured, tracing works locally only
