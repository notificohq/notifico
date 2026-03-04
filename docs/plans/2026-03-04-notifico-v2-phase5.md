# Phase 5: Credential Storage & Email Transport

## Goal
Add encrypted credential storage, wire credentials into the delivery pipeline,
and implement the first real transport: email via SMTP (lettre).

## Architecture

```
Credential flow:
  Admin stores SMTP creds → AES-256-GCM encrypted → credential table
  Worker processes task → resolve credential by project+channel → decrypt
  → pass to transport.send() via RenderedMessage.credentials

Email transport:
  RenderedMessage { content: {subject, text, html?}, credentials: {host, port, username, password} }
  → lettre SmtpTransport → MIME multipart/alternative (text + html)
```

## Depends on
- Phase 4 complete (queue, worker loop, console transport) ✓

---

## Task 24: Add credential table migration

### Steps

1. Create `notifico-db/src/migration/m20260304_000009_create_credentials.rs`:

```rust
// Table: credential
// Columns:
//   id: UUID PK
//   project_id: UUID FK → project
//   name: VARCHAR(255) NOT NULL (e.g. "Production SMTP", "Twilio Prod")
//   channel: VARCHAR(64) NOT NULL (e.g. "email", "sms")
//   encrypted_data: TEXT NOT NULL (base64-encoded AES-256-GCM ciphertext)
//   enabled: BOOLEAN DEFAULT true
//   created_at: TIMESTAMP DEFAULT CURRENT_TIMESTAMP
//   updated_at: TIMESTAMP DEFAULT CURRENT_TIMESTAMP
//
// Indexes:
//   idx_credential_project_channel (project_id, channel)
//
// Unique: (project_id, name)
```

2. Register in migration/mod.rs.

### Verify
```
cargo test -p notifico-db
```

---

## Task 25: Add credential repository with encryption

### Steps

1. Add workspace deps to root `Cargo.toml`:
   - `aes-gcm = "0.10"` under `# Crypto`
   - `base64 = "0.22"` under `# Crypto`
   - `rand = "0.9"` under `# Crypto`

2. Add to `notifico-db/Cargo.toml`: `aes-gcm`, `base64`, `rand`, `serde_json`

3. Create `notifico-db/src/repo/credential.rs`:

```rust
// Encryption: AES-256-GCM
// - Key: 32 bytes from hex-encoded NOTIFICO_ENCRYPTION_KEY env var
// - Nonce: random 12 bytes, prepended to ciphertext
// - Stored as: base64(nonce || ciphertext)

pub fn encrypt_credential(data: &Value, key: &[u8; 32]) -> Result<String, DbErr>
pub fn decrypt_credential(encrypted: &str, key: &[u8; 32]) -> Result<Value, DbErr>

pub async fn insert_credential(
    db: &DatabaseConnection,
    id: Uuid, project_id: Uuid, name: &str, channel: &str,
    data: &Value, key: &[u8; 32],
) -> Result<(), DbErr>

pub async fn find_credential(
    db: &DatabaseConnection,
    project_id: Uuid, channel: &str, key: &[u8; 32],
) -> Result<Option<CredentialRow>, DbErr>
// Returns first enabled credential for project+channel, decrypted

pub struct CredentialRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub channel: String,
    pub data: Value,  // decrypted
    pub enabled: bool,
}
```

### Tests
- `encrypt_decrypt_roundtrip`: encrypt JSON, decrypt, verify equal
- `insert_and_find_credential`: store encrypted, find by project+channel, verify decrypted fields
- `find_returns_none_when_missing`: no credential for channel → None
- `disabled_credential_skipped`: disabled credential not returned by find_credential

### Verify
```
cargo test -p notifico-db
```

---

## Task 26: Wire credentials into worker delivery

### Steps

1. Add `encryption_key: Option<[u8; 32]>` to `AppState` in main.rs:
   - Parse from `config.auth.encryption_key` (hex string → 32 bytes)
   - None if not configured (console transport doesn't need creds)

2. Update `worker.rs` `process_delivery` to accept optional encryption key:
   - Before calling transport.send(), resolve credential from DB
   - If found, set `message.credentials = credential.data`
   - If not found and transport.credential_schema().fields has required fields → error

3. Update `run_worker_loop` to pass encryption key to process_delivery.

### Verify
```
cargo test --workspace
```

---

## Task 27: Add email transport via lettre

### Steps

1. Add workspace deps:
   - `lettre = { version = "0.11", features = ["tokio1-rustls-tls", "smtp-transport", "builder"] }`

2. Create `notifico-core/src/transport/email.rs` (refactor transport.rs into module):
   - Move types from `transport.rs` → `transport/mod.rs`
   - Create `transport/email.rs`

```rust
pub struct EmailTransport;

impl Transport for EmailTransport {
    fn channel_id() -> "email"
    fn display_name() -> "Email (SMTP)"

    fn content_schema() -> ContentSchema {
        // Required: subject (text), text (text)
        // Optional: html (html)
    }

    fn credential_schema() -> CredentialSchema {
        // Required: smtp_host, smtp_port, smtp_username, smtp_password
        // Optional: from_address, from_name, tls (bool)
    }

    async fn send(message: &RenderedMessage) -> Result<DeliveryResult, CoreError> {
        // 1. Extract credentials: host, port, username, password, from
        // 2. Extract content: subject, text, html (optional)
        // 3. Build lettre Message:
        //    - From: from_address or smtp_username
        //    - To: message.recipient_contact
        //    - Subject: subject
        //    - multipart/alternative: text + html (if present)
        // 4. Build SmtpTransport with STARTTLS
        // 5. Send, return Delivered or Failed
    }
}
```

3. Add `lettre` to `notifico-core/Cargo.toml`.

### Tests
- `email_content_schema`: verify required fields (subject, text)
- `email_credential_schema`: verify required fields (smtp_host, etc.)
- `email_channel_id`: verify "email"
- (No actual SMTP test — that requires a real server. We test the schema/config.)

### Verify
```
cargo test -p notifico-core
```

---

## Task 28: Register email transport and update integration test

### Steps

1. In `main.rs`, register EmailTransport alongside ConsoleTransport:
   ```rust
   registry.register(Arc::new(ConsoleTransport));
   registry.register(Arc::new(EmailTransport));
   ```

2. Update integration test in main.rs to also seed a credential for the
   email channel (so end-to-end flow works with email transport).
   But since we don't have a real SMTP server in tests, keep the
   integration test using `console` channel or mock.

3. Add a new unit test in worker.rs or a new integration test that verifies
   credential resolution works (use SQLite in-memory + seeded credential).

### Verify
```
cargo test --workspace
```

---

## Summary

| Task | Description | New tests |
|------|-------------|-----------|
| 24 | Credential table migration | migration runs ✓ |
| 25 | Credential repo with AES-256-GCM | 4 tests |
| 26 | Wire credentials into worker | existing tests pass |
| 27 | Email transport (lettre SMTP) | 3 tests |
| 28 | Register + integration | existing tests pass |

After Phase 5, the system can send real emails via SMTP with encrypted
credential storage. Phase 6 would add webhook transport, admin API for
managing credentials/templates, and MJML rendering.
