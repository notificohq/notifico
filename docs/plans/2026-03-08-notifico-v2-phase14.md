# Phase 14: Frontend — SvelteKit Admin Panel

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a SvelteKit admin panel SPA with full CRUD for all Notifico resources, embedded in the Rust binary.

**Architecture:** SvelteKit SPA (adapter-static) with TypeScript, shadcn-svelte, TanStack Query, Tailwind. Embedded in Rust via rust-embed.

**Tech Stack:** Svelte 5, SvelteKit, TypeScript, shadcn-svelte, TanStack Query, Tailwind CSS, Bun

**Design doc:** `docs/plans/2026-03-08-notifico-v2-frontend-design.md`

---

## Task 64: Scaffold SvelteKit project

**Files:**
- Create: `notifico-frontend/` — full SvelteKit project

**Step 1: Initialize SvelteKit project with Bun**

```bash
cd /mnt/devenv/workspace/notifico/notifico
bunx sv create notifico-frontend --template minimal --types ts --no-add-ons --no-install
cd notifico-frontend
bun install
```

**Step 2: Install dependencies**

```bash
bun add -d @sveltejs/adapter-static
bun add -d tailwindcss @tailwindcss/vite
bun add @tanstack/svelte-query
```

**Step 3: Configure adapter-static**

Update `svelte.config.js`:
```js
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      precompress: false,
    }),
  },
};
```

**Step 4: Configure Tailwind**

Update `vite.config.ts`:
```ts
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
});
```

Add to `src/app.css`:
```css
@import 'tailwindcss';
```

**Step 5: Add SPA layout**

Create `src/routes/+layout.ts`:
```ts
export const prerender = true;
export const ssr = false;
```

**Step 6: Verify build**

```bash
bun run build
```

**Step 7: Commit**

```
feat: scaffold SvelteKit frontend with Tailwind and adapter-static
```

---

## Task 65: Install shadcn-svelte

**Step 1: Initialize shadcn-svelte**

```bash
cd notifico-frontend
bunx shadcn-svelte@next init
```

Follow prompts: select defaults, Tailwind, etc.

**Step 2: Add core components**

```bash
bunx shadcn-svelte@next add button input card table badge dialog alert-dropdown-menu sidebar sheet separator label select textarea tabs toast sonner
```

**Step 3: Verify build**

```bash
bun run build
```

**Step 4: Commit**

```
feat: add shadcn-svelte UI components
```

---

## Task 66: API client and auth store

**Files:**
- Create: `notifico-frontend/src/lib/api/client.ts`
- Create: `notifico-frontend/src/lib/stores/auth.ts`

**Step 1: Create typed API client**

`src/lib/api/client.ts`:
```ts
import { authStore } from '$lib/stores/auth';
import { get } from 'svelte/store';
import { goto } from '$app/navigation';

const BASE_URL = '';

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const auth = get(authStore);
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...((options.headers as Record<string, string>) || {}),
  };

  if (auth.apiKey) {
    headers['Authorization'] = `Bearer ${auth.apiKey}`;
  }

  const resp = await fetch(`${BASE_URL}${path}`, { ...options, headers });

  if (resp.status === 401) {
    authStore.logout();
    goto('/login');
    throw new Error('Unauthorized');
  }

  if (!resp.ok) {
    const text = await resp.text();
    throw new Error(`API error ${resp.status}: ${text}`);
  }

  if (resp.status === 204) return undefined as T;
  return resp.json();
}

export const api = {
  get: <T>(path: string) => request<T>(path),
  post: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'POST', body: JSON.stringify(body) }),
  put: <T>(path: string, body: unknown) =>
    request<T>(path, { method: 'PUT', body: JSON.stringify(body) }),
  delete: <T>(path: string) => request<T>(path, { method: 'DELETE' }),
};
```

**Step 2: Create auth store**

`src/lib/stores/auth.ts`:
```ts
import { writable } from 'svelte/store';

interface AuthState {
  apiKey: string | null;
  isAuthenticated: boolean;
}

function createAuthStore() {
  const stored = typeof localStorage !== 'undefined'
    ? localStorage.getItem('notifico_api_key')
    : null;

  const { subscribe, set } = writable<AuthState>({
    apiKey: stored,
    isAuthenticated: !!stored,
  });

  return {
    subscribe,
    login: (apiKey: string) => {
      localStorage.setItem('notifico_api_key', apiKey);
      set({ apiKey, isAuthenticated: true });
    },
    logout: () => {
      localStorage.removeItem('notifico_api_key');
      set({ apiKey: null, isAuthenticated: false });
    },
  };
}

export const authStore = createAuthStore();
```

**Step 3: Commit**

```
feat: add API client and auth store
```

---

## Task 67: Login page

**Files:**
- Create: `notifico-frontend/src/routes/login/+page.svelte`

**Step 1: Build login page**

Simple form: API key input + submit button. On submit, try `GET /admin/api/v1/projects` with the key. If 200, call `authStore.login(key)` and redirect to `/dashboard`. If error, show error message.

**Step 2: Commit**

```
feat: add login page with API key auth
```

---

## Task 68: App layout with sidebar

**Files:**
- Modify: `notifico-frontend/src/routes/+layout.svelte`
- Create: `notifico-frontend/src/lib/components/Sidebar.svelte`

**Step 1: Build sidebar layout**

- Sidebar with nav links: Dashboard, Events, Templates, Recipients, Credentials, Delivery Log, Broadcasts, API Keys, Settings
- Auth guard: redirect to `/login` if not authenticated
- Top bar with logout button
- Collapsible sidebar on mobile
- Use shadcn-svelte Sidebar component

**Step 2: Commit**

```
feat: add app layout with sidebar navigation
```

---

## Task 69: Dashboard page

**Files:**
- Create: `notifico-frontend/src/routes/dashboard/+page.svelte`
- Create: `notifico-frontend/src/lib/api/events.ts`
- Create: `notifico-frontend/src/lib/api/delivery-log.ts`

**Step 1: Build dashboard**

- Recent delivery log (last 20 entries) in a table
- Show total counts by status if available
- Use TanStack Query for data fetching
- Link to event detail for stats

**Step 2: Commit**

```
feat: add dashboard page with delivery overview
```

---

## Task 70: Events page + pipeline rules

**Files:**
- Create: `notifico-frontend/src/routes/events/+page.svelte`
- Create: `notifico-frontend/src/routes/events/[id]/+page.svelte`

**Step 1: Events list page**

- Table of events with name, category columns
- Create event dialog (name + category)
- Delete button per event

**Step 2: Event detail page**

- Event info header
- Pipeline rules table (channel, template, priority, enabled)
- Add/edit/delete rules
- Middleware configuration per rule (list, add, configure, reorder, delete)
- Event stats (delivery counts by status)

**Step 3: Commit**

```
feat: add events and pipeline rules pages
```

---

## Task 71: Templates page

**Files:**
- Create: `notifico-frontend/src/routes/templates/+page.svelte`
- Create: `notifico-frontend/src/routes/templates/[id]/+page.svelte`

**Step 1: Templates list**

- Table with name, channel columns
- Create template dialog (name + channel)
- Delete button

**Step 2: Template detail/editor**

- Tab per locale (show existing, add new)
- JSON editor for template body
- Live preview: sends preview request, shows rendered output
- Save button per locale

**Step 3: Commit**

```
feat: add templates list and editor pages
```

---

## Task 72: Recipients page

**Files:**
- Create: `notifico-frontend/src/routes/recipients/+page.svelte`
- Create: `notifico-frontend/src/routes/recipients/[id]/+page.svelte`

**Step 1: Recipients list**

- Table with external_id, locale columns
- Create/delete recipients

**Step 2: Recipient detail**

- Contacts list (channel + value) with add/delete
- Preferences list with toggle

**Step 3: Commit**

```
feat: add recipients and contacts pages
```

---

## Task 73: Credentials, Delivery Log, Broadcasts, API Keys, Settings pages

**Files:**
- Create: `notifico-frontend/src/routes/credentials/+page.svelte`
- Create: `notifico-frontend/src/routes/delivery-log/+page.svelte`
- Create: `notifico-frontend/src/routes/broadcasts/+page.svelte`
- Create: `notifico-frontend/src/routes/api-keys/+page.svelte`
- Create: `notifico-frontend/src/routes/settings/+page.svelte`

**Step 1: Credentials page**

- Table with name, channel, enabled columns
- Create credential dialog (name, channel, JSON config)
- Delete button

**Step 2: Delivery log page**

- Filterable table (status, event_name)
- Pagination
- Auto-refresh toggle

**Step 3: Broadcasts page**

- Form: select event, JSON data editor, optional filter
- Submit sends POST /api/v1/broadcasts
- Show result (broadcast_id, recipient_count, task_count)

**Step 4: API Keys page**

- Table with name, scope, prefix, created_at
- Create key dialog (shows raw key once)
- Delete button

**Step 5: Settings page**

- Project list with default_locale
- Available channels with schemas (from GET /channels)

**Step 6: Commit**

```
feat: add credentials, delivery log, broadcasts, API keys, and settings pages
```

---

## Task 74: Embed frontend in Rust binary

**Files:**
- Modify: `Cargo.toml` — add rust-embed workspace dep
- Modify: `notifico-server/Cargo.toml` — add rust-embed
- Modify: `notifico-server/src/main.rs` — add static file serving with SPA fallback

**Step 1: Add rust-embed**

Root `Cargo.toml`:
```toml
rust-embed = { version = "8", features = ["interpolate-folder-path"] }
```

**Step 2: Embed frontend build output**

In `notifico-server/src/main.rs`:
```rust
#[derive(rust_embed::Embed)]
#[folder = "../notifico-frontend/build/"]
struct FrontendAssets;
```

Add fallback handler that serves embedded files, with `index.html` fallback for SPA routes.

**Step 3: Add to router**

Mount as fallback after all API routes.

**Step 4: Update Dockerfile**

Add bun install + build step before cargo build.

**Step 5: Commit**

```
feat: embed SvelteKit frontend in Rust binary via rust-embed
```
