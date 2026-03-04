use aes_gcm::aead::{Aead, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CredentialRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub channel: String,
    pub data: Value,
    pub enabled: bool,
}

#[derive(Debug, Clone, FromQueryResult)]
struct CredentialRaw {
    id: String,
    project_id: String,
    name: String,
    channel: String,
    encrypted_data: String,
    enabled: bool,
}

impl CredentialRaw {
    fn into_row(self, key: &[u8; 32]) -> Result<CredentialRow, DbErr> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DbErr::Custom(format!("invalid id UUID: {e}")))?;
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| DbErr::Custom(format!("invalid project_id UUID: {e}")))?;
        let data = decrypt_credential(&self.encrypted_data, key)?;
        Ok(CredentialRow {
            id,
            project_id,
            name: self.name,
            channel: self.channel,
            data,
            enabled: self.enabled,
        })
    }
}

/// Encrypt credential data with AES-256-GCM.
/// Returns base64(nonce || ciphertext).
pub fn encrypt_credential(data: &Value, key: &[u8; 32]) -> Result<String, DbErr> {
    let plaintext = serde_json::to_vec(data)
        .map_err(|e| DbErr::Custom(format!("JSON serialize error: {e}")))?;

    let cipher = Aes256Gcm::new(key.into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|e| DbErr::Custom(format!("Encryption error: {e}")))?;

    // Prepend nonce (12 bytes) to ciphertext
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

/// Decrypt credential data from base64(nonce || ciphertext).
pub fn decrypt_credential(encrypted: &str, key: &[u8; 32]) -> Result<Value, DbErr> {
    let combined = BASE64
        .decode(encrypted)
        .map_err(|e| DbErr::Custom(format!("Base64 decode error: {e}")))?;

    if combined.len() < 13 {
        return Err(DbErr::Custom("Encrypted data too short".into()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = aes_gcm::Nonce::from_slice(nonce_bytes);
    let cipher = Aes256Gcm::new(key.into());

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| DbErr::Custom(format!("Decryption error: {e}")))?;

    serde_json::from_slice(&plaintext)
        .map_err(|e| DbErr::Custom(format!("JSON deserialize error: {e}")))
}

/// Insert an encrypted credential.
pub async fn insert_credential(
    db: &DatabaseConnection,
    id: Uuid,
    project_id: Uuid,
    name: &str,
    channel: &str,
    data: &Value,
    key: &[u8; 32],
) -> Result<(), DbErr> {
    let encrypted = encrypt_credential(data, key)?;

    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO credential (id, project_id, name, channel, encrypted_data) VALUES (?, ?, ?, ?, ?)",
        [
            id.to_string().into(),
            project_id.to_string().into(),
            name.into(),
            channel.into(),
            encrypted.into(),
        ],
    ))
    .await?;

    Ok(())
}

/// Find the first enabled credential for a project + channel, decrypted.
pub async fn find_credential(
    db: &DatabaseConnection,
    project_id: Uuid,
    channel: &str,
    key: &[u8; 32],
) -> Result<Option<CredentialRow>, DbErr> {
    let raw = CredentialRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, project_id, name, channel, encrypted_data, enabled FROM credential WHERE project_id = ? AND channel = ? AND enabled = true LIMIT 1",
        [project_id.to_string().into(), channel.into()],
    ))
    .one(db)
    .await?;

    match raw {
        Some(r) => Ok(Some(r.into_row(key)?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};
    use sea_orm::ConnectionTrait;
    use serde_json::json;

    fn test_key() -> [u8; 32] {
        [0xABu8; 32]
    }

    async fn setup() -> DatabaseConnection {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        let project_id = "00000000-0000-0000-0000-000000000001";
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();
        db
    }

    fn test_project_id() -> Uuid {
        Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let key = test_key();
        let data = json!({"smtp_host": "mail.example.com", "smtp_port": 587, "smtp_password": "secret123"});

        let encrypted = encrypt_credential(&data, &key).unwrap();
        assert_ne!(encrypted, serde_json::to_string(&data).unwrap());

        let decrypted = decrypt_credential(&encrypted, &key).unwrap();
        assert_eq!(decrypted, data);
    }

    #[tokio::test]
    async fn insert_and_find_credential() {
        let db = setup().await;
        let key = test_key();
        let cred_id = Uuid::now_v7();
        let data = json!({"smtp_host": "smtp.example.com", "smtp_port": 587, "smtp_username": "user", "smtp_password": "pass"});

        insert_credential(
            &db, cred_id, test_project_id(), "Production SMTP", "email", &data, &key,
        )
        .await
        .unwrap();

        let found = find_credential(&db, test_project_id(), "email", &key)
            .await
            .unwrap()
            .expect("Should find credential");

        assert_eq!(found.id, cred_id);
        assert_eq!(found.name, "Production SMTP");
        assert_eq!(found.channel, "email");
        assert_eq!(found.data, data);
        assert!(found.enabled);
    }

    #[tokio::test]
    async fn find_returns_none_when_missing() {
        let db = setup().await;
        let key = test_key();

        let found = find_credential(&db, test_project_id(), "sms", &key)
            .await
            .unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn disabled_credential_skipped() {
        let db = setup().await;
        let key = test_key();
        let cred_id = Uuid::now_v7();

        insert_credential(
            &db, cred_id, test_project_id(), "Disabled SMTP", "email",
            &json!({"host": "smtp.example.com"}), &key,
        )
        .await
        .unwrap();

        // Disable it
        db.execute_unprepared(&format!(
            "UPDATE credential SET enabled = false WHERE id = '{cred_id}'"
        ))
        .await
        .unwrap();

        let found = find_credential(&db, test_project_id(), "email", &key)
            .await
            .unwrap();
        assert!(found.is_none());
    }
}
