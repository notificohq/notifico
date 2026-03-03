# Notifico v2 — Differentiation & In-App Inbox Design

**Date:** 2026-03-03
**Status:** Approved
**Extends:** `2026-03-03-notifico-v2-design.md`

---

## Differentiation Strategy

Notifico competes with Novu, Knock, MagicBell, SuprSend. Architectural differentiation
is baked in from day one — not bolted on later.

### 1. Lightweight Single-Binary

Novu requires MongoDB + 2 Redis clusters + multiple Node.js services (recommended:
36 vCPUs, 64GB RAM for production). Notifico runs as a single Rust binary.

- **Dev/small deploy:** single binary + SQLite. Zero infrastructure. Runs on Raspberry Pi.
- **Production:** same binary + PostgreSQL + Valkey. Horizontal scaling via `--mode api/worker`.
- **No runtime dependencies:** no Node.js, no JVM, no MongoDB.
- **Memory:** ~20-50MB idle vs Novu's ~2GB+ minimum.

This is the strongest self-hosted story in the market.

### 2. Russian-Language Messengers (Telegram Full + MAX)

No competitor has native VK MAX support. Novu's Telegram integration is minimal.

**`notifico-telegram`** — full Telegram Bot API:
- Parse modes: MarkdownV2, HTML
- Inline keyboards (`InlineKeyboardMarkup`)
- Reply keyboards
- Media: photo, document, video, audio
- Message editing and deletion
- Content schema: `text`, `parse_mode`, `reply_markup`, `media`

**`notifico-max`** — VK MAX Bot API:
- Text messages, buttons, media
- Content schema: `text`, `buttons`, `media`

Both are first-class transports with full feature sets, not minimal wrappers.

### 3. Template Versioning + Multilingual

Already in the data model (approved design). No competitor offers both natively:
- Template → versions → content per locale
- Rollback to any previous version
- Preview per locale in admin panel
- Fallback chain: recipient locale → project default locale

---

## In-App Inbox

### Overview

Inbox is a first-class notification channel. It registers as a Transport (`channel_id: "inbox"`)
in the unified pipeline. The same event can route to email + inbox + telegram simultaneously
via pipeline rules.

Unlike external transports, inbox `send()` writes to a local DB table and publishes
to Valkey pub/sub for real-time delivery via WebSocket.

### Data Model

```
inbox_message
+-- id: UUID v7 (sortable by creation time)
+-- project_id: UUID
+-- recipient_id: UUID
+-- event_name: String
+-- title: String
+-- body: String (Markdown, CommonMark subset)
+-- redirect_url: String?
+-- tags: String[] (for feed filtering / tabs in UI)
+-- actions: JSONB  -- [{ label: String, url: String, style: "primary"|"secondary"|"danger"|"link" }]
+-- data: JSONB     -- arbitrary client data (avatar, icon, color, custom fields)
+-- read_at: TimestampTZ?
+-- seen_at: TimestampTZ?
+-- archived_at: TimestampTZ?
+-- created_at: TimestampTZ
```

Indexes:
- `(project_id, recipient_id, created_at DESC)` — main feed query
- `(project_id, recipient_id, read_at)` — unread count
- `(project_id, recipient_id, archived_at)` — exclude archived

### Transport Implementation

```rust
struct InboxTransport { db: DatabaseConnection, publisher: ValkePublisher }

impl Transport for InboxTransport {
    fn channel_id(&self) -> &str { "inbox" }
    fn display_name(&self) -> &str { "In-App Inbox" }

    fn content_schema(&self) -> ContentSchema {
        // title (required), body (Markdown), redirect_url, tags, actions, data
    }

    async fn send(&self, message: RenderedMessage) -> Result<DeliveryResult> {
        // 1. Insert into inbox_message table
        // 2. Publish to Valkey channel "inbox:{recipient_id}"
        // 3. Return DeliveryResult::Delivered
    }
}
```

### Real-Time: WebSocket

**Connection:**
```
ws://notifico.example.com/api/v1/inbox/ws?token={subscriber_token}
```

Token is a short-lived JWT or HMAC-signed token issued by the client application's backend
via the Notifico API. This prevents end-users from forging connections.

**Server → Client messages:**
```json
{ "type": "notification", "data": { /* InboxMessage */ } }
{ "type": "count_update", "unread": 5, "unseen": 12 }
```

**Client → Server messages:**
```json
{ "type": "mark_read", "ids": ["uuid1", "uuid2"] }
{ "type": "mark_seen", "ids": ["uuid1"] }
{ "type": "mark_all_read" }
{ "type": "archive", "ids": ["uuid1"] }
```

**Implementation:** `tokio-tungstenite` via axum's built-in WebSocket support.
Each connected client subscribes to Valkey pub/sub channel `inbox:{project_id}:{recipient_id}`.
When a worker delivers an inbox notification, it publishes to the channel. All connected
API nodes receive the message and forward to the appropriate WebSocket connection.

**Scaling:** In `--mode all`, pub/sub is in-process. In `--mode api` with multiple replicas,
Valkey pub/sub fans out to all API nodes. No sticky sessions needed.

### REST API (History + Actions)

Part of Public API (authn: API key, scope `public`, or subscriber token):

```
GET  /api/v1/inbox/messages?tag=orders&status=unread&limit=20&cursor=...
GET  /api/v1/inbox/messages/count   -- { unread: 5, unseen: 12, total: 100 }
POST /api/v1/inbox/messages/{id}/read
POST /api/v1/inbox/messages/{id}/unread
POST /api/v1/inbox/messages/{id}/archive
POST /api/v1/inbox/messages/read-all
POST /api/v1/inbox/messages/archive-all?tag=marketing
```

Cursor-based pagination using UUID v7 (naturally sorted by time).

### Client SDKs

#### `@notifico/js` (headless, framework-agnostic)

```typescript
import { NotificoInbox } from '@notifico/js';

const inbox = new NotificoInbox({
  endpoint: 'https://notifico.example.com',
  subscriberToken: 'token_xxx',
});

// Real-time events
inbox.on('notification', (msg) => { /* new notification arrived */ });
inbox.on('count_update', ({ unread, unseen }) => { /* update badge */ });
inbox.on('connected', () => {});
inbox.on('disconnected', () => {});

// Fetch history (cursor-based)
const { messages, nextCursor } = await inbox.fetchMessages({
  tag: 'orders',
  status: 'unread',
  limit: 20,
});

// Actions
await inbox.markRead('msg-uuid');
await inbox.markAllRead();
await inbox.archive('msg-uuid');

// Counts
const counts = await inbox.getCounts(); // { unread, unseen, total }

// Cleanup
inbox.disconnect();
```

Handles: WebSocket connection/reconnection, state sync, optimistic updates.

#### `@notifico/react` (UI kit)

```tsx
import { NotificoProvider, InboxBell, InboxFeed, InboxPreferences } from '@notifico/react';

function App() {
  return (
    <NotificoProvider endpoint="..." subscriberToken="...">
      <InboxBell />          {/* bell icon + unread badge */}
      <InboxFeed />          {/* notification list with tabs */}
      <InboxPreferences />   {/* channel preference panel */}
    </NotificoProvider>
  );
}
```

Headless hooks for custom UI:
- `useNotifications({ tag?, status?, limit? })` — paginated feed
- `useUnreadCount()` — real-time unread count
- `useNotifico()` — direct access to `NotificoInbox` instance

Theming via CSS variables. Dark mode built-in.

### Feeds (Tags/Tabs)

Tags are set per inbox message (from template or pipeline rule configuration).
SDK filters by tag for tabbed UI:

```tsx
<InboxFeed tabs={[
  { label: 'All', tag: undefined },
  { label: 'Orders', tag: 'orders' },
  { label: 'Marketing', tag: 'marketing' },
]} />
```

Tags are strings, not predefined. Admin defines them per pipeline rule or template.

### Subscriber Token

End-user authentication for inbox uses a subscriber token — a short-lived token
generated by the client application's backend:

```
POST /api/v1/inbox/tokens
{
  "recipient_id": "user-123",
  "ttl": 3600
}
```

Returns `{ "token": "nk_sub_..." }`. This token is passed to the JS SDK.
Prevents end-users from accessing other users' inboxes. Token is validated
on WebSocket connect and REST API calls.

---

## Updated Crate Structure

Add to the crate structure from the main design doc:

```
notifico/
+-- ...existing crates...
+-- notifico-inbox/              # Inbox transport + WS handler + REST API
+-- sdks/
|   +-- js/                      # @notifico/js (TypeScript, framework-agnostic)
|   +-- react/                   # @notifico/react (React UI kit)
```

---

## Updated Transport Table

| Crate | Channel ID | Protocol | Content Schema |
|-------|-----------|----------|----------------|
| ...existing... | | | |
| `notifico-inbox` | `inbox` | Local DB + WS | title, body (md), redirect_url, tags, actions, data |

---

## Updated Roadmap

Add to post-v1 roadmap:
- **WebTransport** — alternative to WebSocket when browser/proxy support matures
- **Vue/Svelte SDK** — UI kits for other frameworks (headless JS SDK covers all)
- **Mobile SDKs** — React Native, Flutter wrappers around `@notifico/js`
- **Digest/batching** — combine multiple inbox notifications into one
