use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Result of looking up an API key.
#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub scope: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, FromQueryResult)]
struct ApiKeyRaw {
    id: String,
    project_id: String,
    name: String,
    scope: String,
    enabled: bool,
}

impl ApiKeyRaw {
    fn into_info(self) -> Result<ApiKeyInfo, DbErr> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DbErr::Custom(format!("invalid id UUID: {e}")))?;
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| DbErr::Custom(format!("invalid project_id UUID: {e}")))?;
        Ok(ApiKeyInfo {
            id,
            project_id,
            name: self.name,
            scope: self.scope,
            enabled: self.enabled,
        })
    }
}

/// Hash a raw API key with SHA-256 (same algorithm used when creating keys).
pub fn hash_api_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Look up an API key by its raw value. Hashes it and queries the DB.
pub async fn find_by_raw_key(
    db: &DatabaseConnection,
    raw_key: &str,
) -> Result<Option<ApiKeyInfo>, DbErr> {
    let key_hash = hash_api_key(raw_key);

    let raw = ApiKeyRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, project_id, name, scope, enabled FROM api_key WHERE key_hash = ?",
        [key_hash.into()],
    ))
    .one(db)
    .await?;

    match raw {
        Some(r) => Ok(Some(r.into_info()?)),
        None => Ok(None),
    }
}

/// Insert an API key (for testing / admin). Stores the SHA-256 hash.
pub async fn insert_api_key(
    db: &DatabaseConnection,
    id: Uuid,
    project_id: Uuid,
    name: &str,
    raw_key: &str,
    scope: &str,
) -> Result<(), DbErr> {
    let key_hash = hash_api_key(raw_key);
    let prefix = &raw_key[..raw_key.len().min(16)];

    db.execute_unprepared(&format!(
        "INSERT INTO api_key (id, project_id, name, key_hash, key_prefix, scope) \
         VALUES ('{id}', '{project_id}', '{name}', '{key_hash}', '{prefix}', '{scope}')"
    ))
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};

    async fn setup_db() -> DatabaseConnection {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();
        db
    }

    async fn seed_project(db: &DatabaseConnection) -> Uuid {
        let project_id = Uuid::now_v7();
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();
        project_id
    }

    #[tokio::test]
    async fn hash_and_find_api_key() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;
        let key_id = Uuid::now_v7();
        let raw_key = "nk_live_test1234567890abcdef";

        insert_api_key(&db, key_id, project_id, "Test Key", raw_key, "ingest")
            .await
            .unwrap();

        let found = find_by_raw_key(&db, raw_key).await.unwrap().unwrap();
        assert_eq!(found.id, key_id);
        assert_eq!(found.project_id, project_id);
        assert_eq!(found.scope, "ingest");
        assert!(found.enabled);
    }

    #[tokio::test]
    async fn find_nonexistent_key_returns_none() {
        let db = setup_db().await;
        let found = find_by_raw_key(&db, "nk_live_doesnotexist").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn disabled_key_still_found() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;
        let key_id = Uuid::now_v7();
        let raw_key = "nk_live_disabled_key_12345";

        insert_api_key(&db, key_id, project_id, "Disabled", raw_key, "ingest")
            .await
            .unwrap();

        // Disable it
        db.execute_unprepared(&format!(
            "UPDATE api_key SET enabled = false WHERE id = '{key_id}'"
        ))
        .await
        .unwrap();

        let found = find_by_raw_key(&db, raw_key).await.unwrap().unwrap();
        assert!(!found.enabled);
    }
}
