# Notifico v2 Frontend Design

## Overview

Admin panel SPA for Notifico, providing full CRUD management of all backend resources.

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
