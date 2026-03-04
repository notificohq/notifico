use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use serde_json::Value;
use uuid::Uuid;

/// A recipient looked up from DB.
#[derive(Debug, Clone)]
pub struct RecipientRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub external_id: String,
    pub locale: String,
    pub timezone: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, FromQueryResult)]
struct RecipientRaw {
    id: String,
    project_id: String,
    external_id: String,
    locale: String,
    timezone: String,
    metadata: Value,
}

impl RecipientRaw {
    fn into_row(self) -> Result<RecipientRow, DbErr> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DbErr::Custom(format!("invalid id UUID: {e}")))?;
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| DbErr::Custom(format!("invalid project_id UUID: {e}")))?;
        Ok(RecipientRow {
            id,
            project_id,
            external_id: self.external_id,
            locale: self.locale,
            timezone: self.timezone,
            metadata: self.metadata,
        })
    }
}

/// Look up a recipient by project_id + external_id.
pub async fn find_by_external_id(
    db: &DatabaseConnection,
    project_id: Uuid,
    external_id: &str,
) -> Result<Option<RecipientRow>, DbErr> {
    let raw = RecipientRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, project_id, external_id, locale, timezone, metadata \
         FROM recipient WHERE project_id = ? AND external_id = ?",
        [project_id.to_string().into(), external_id.into()],
    ))
    .one(db)
    .await?;

    match raw {
        Some(r) => Ok(Some(r.into_row()?)),
        None => Ok(None),
    }
}

/// A contact value for a specific channel.
#[derive(Debug, Clone)]
pub struct ContactRow {
    pub id: Uuid,
    pub channel: String,
    pub value: String,
    pub verified: bool,
}

#[derive(Debug, Clone, FromQueryResult)]
struct ContactRaw {
    id: String,
    channel: String,
    value: String,
    verified: bool,
}

/// Get all contacts for a recipient.
pub async fn get_contacts(
    db: &DatabaseConnection,
    recipient_id: Uuid,
) -> Result<Vec<ContactRow>, DbErr> {
    let rows = ContactRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, channel, value, verified FROM recipient_contact WHERE recipient_id = ?",
        [recipient_id.to_string().into()],
    ))
    .all(db)
    .await?;

    rows.into_iter()
        .map(|r| {
            let id = Uuid::parse_str(&r.id)
                .map_err(|e| DbErr::Custom(format!("invalid contact id UUID: {e}")))?;
            Ok(ContactRow {
                id,
                channel: r.channel,
                value: r.value,
                verified: r.verified,
            })
        })
        .collect()
}

/// Upsert a recipient (insert or return existing by project_id + external_id).
/// Returns the recipient's internal UUID.
pub async fn upsert_recipient(
    db: &DatabaseConnection,
    project_id: Uuid,
    external_id: &str,
    locale: Option<&str>,
) -> Result<Uuid, DbErr> {
    if let Some(existing) = find_by_external_id(db, project_id, external_id).await? {
        return Ok(existing.id);
    }

    let id = Uuid::now_v7();
    let locale = locale.unwrap_or("en");

    db.execute_unprepared(&format!(
        "INSERT INTO recipient (id, project_id, external_id, locale) \
         VALUES ('{id}', '{project_id}', '{external_id}', '{locale}')"
    ))
    .await?;

    Ok(id)
}

/// Upsert a contact for a recipient (insert if not exists by recipient_id + channel + value).
pub async fn upsert_contact(
    db: &DatabaseConnection,
    recipient_id: Uuid,
    channel: &str,
    value: &str,
) -> Result<(), DbErr> {
    #[derive(FromQueryResult)]
    struct IdOnly {
        #[allow(dead_code)]
        id: String,
    }

    let existing = IdOnly::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM recipient_contact WHERE recipient_id = ? AND channel = ? AND value = ?",
        [
            recipient_id.to_string().into(),
            channel.into(),
            value.into(),
        ],
    ))
    .one(db)
    .await?;

    if existing.is_some() {
        return Ok(());
    }

    let id = Uuid::now_v7();
    db.execute_unprepared(&format!(
        "INSERT INTO recipient_contact (id, recipient_id, channel, value) \
         VALUES ('{id}', '{recipient_id}', '{channel}', '{value}')"
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
    async fn upsert_and_find_recipient() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;

        let id = upsert_recipient(&db, project_id, "user-123", Some("ru"))
            .await
            .unwrap();

        let found = find_by_external_id(&db, project_id, "user-123")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(found.id, id);
        assert_eq!(found.locale, "ru");
        assert_eq!(found.external_id, "user-123");
    }

    #[tokio::test]
    async fn upsert_contact_and_get() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;

        let rid = upsert_recipient(&db, project_id, "user-456", None)
            .await
            .unwrap();

        upsert_contact(&db, rid, "email", "test@example.com")
            .await
            .unwrap();
        upsert_contact(&db, rid, "sms", "+1234567890")
            .await
            .unwrap();

        let contacts = get_contacts(&db, rid).await.unwrap();
        assert_eq!(contacts.len(), 2);

        let channels: Vec<&str> = contacts.iter().map(|c| c.channel.as_str()).collect();
        assert!(channels.contains(&"email"));
        assert!(channels.contains(&"sms"));
    }

    #[tokio::test]
    async fn upsert_recipient_idempotent() {
        let db = setup_db().await;
        let project_id = seed_project(&db).await;

        let id1 = upsert_recipient(&db, project_id, "user-789", None)
            .await
            .unwrap();
        let id2 = upsert_recipient(&db, project_id, "user-789", None)
            .await
            .unwrap();

        assert_eq!(id1, id2);
    }
}
