use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PreferenceRow {
    pub id: Uuid,
    pub category: String,
    pub channel: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, FromQueryResult)]
struct PreferenceRaw {
    id: String,
    category: String,
    channel: String,
    enabled: bool,
}

/// List all preferences for a recipient.
pub async fn list_preferences(
    db: &DatabaseConnection,
    recipient_id: Uuid,
) -> Result<Vec<PreferenceRow>, DbErr> {
    let rows = PreferenceRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, category, channel, enabled FROM recipient_preference WHERE recipient_id = ? ORDER BY category, channel",
        [recipient_id.to_string().into()],
    ))
    .all(db)
    .await?;

    rows.into_iter()
        .map(|r| {
            Ok(PreferenceRow {
                id: Uuid::parse_str(&r.id)
                    .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
                category: r.category,
                channel: r.channel,
                enabled: r.enabled,
            })
        })
        .collect()
}

/// Set a preference (upsert by recipient_id + category + channel).
pub async fn set_preference(
    db: &DatabaseConnection,
    recipient_id: Uuid,
    category: &str,
    channel: &str,
    enabled: bool,
) -> Result<(), DbErr> {
    #[derive(FromQueryResult)]
    struct IdOnly {
        #[allow(dead_code)]
        id: String,
    }

    let existing = IdOnly::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM recipient_preference WHERE recipient_id = ? AND category = ? AND channel = ?",
        [
            recipient_id.to_string().into(),
            category.into(),
            channel.into(),
        ],
    ))
    .one(db)
    .await?;

    if existing.is_some() {
        db.execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE recipient_preference SET enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE recipient_id = ? AND category = ? AND channel = ?",
            [
                enabled.into(),
                recipient_id.to_string().into(),
                category.into(),
                channel.into(),
            ],
        ))
        .await?;
    } else {
        let id = Uuid::now_v7();
        db.execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO recipient_preference (id, recipient_id, category, channel, enabled) VALUES (?, ?, ?, ?, ?)",
            [
                id.to_string().into(),
                recipient_id.to_string().into(),
                category.into(),
                channel.into(),
                enabled.into(),
            ],
        ))
        .await?;
    }

    Ok(())
}

/// Check if a recipient has opted out for a specific category+channel.
/// Returns true if opted out (preference exists and enabled=false).
pub async fn is_opted_out(
    db: &DatabaseConnection,
    recipient_id: Uuid,
    category: &str,
    channel: &str,
) -> Result<bool, DbErr> {
    #[derive(FromQueryResult)]
    struct EnabledRow {
        enabled: bool,
    }

    let row = EnabledRow::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT enabled FROM recipient_preference WHERE recipient_id = ? AND category = ? AND channel = ?",
        [
            recipient_id.to_string().into(),
            category.into(),
            channel.into(),
        ],
    ))
    .one(db)
    .await?;

    Ok(row.is_some_and(|r| !r.enabled))
}

/// Create an unsubscribe token for a recipient.
pub async fn create_unsubscribe_token(
    db: &DatabaseConnection,
    recipient_id: Uuid,
    event_id: Option<Uuid>,
    category: Option<&str>,
    channel: Option<&str>,
) -> Result<String, DbErr> {
    let id = Uuid::now_v7();
    let token = format!("unsub_{}", Uuid::now_v7().simple());

    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO unsubscribe (id, recipient_id, event_id, category, channel, token) VALUES (?, ?, ?, ?, ?, ?)",
        [
            id.to_string().into(),
            recipient_id.to_string().into(),
            event_id.map(|e| e.to_string()).into(),
            category.map(|s| s.to_string()).into(),
            channel.map(|s| s.to_string()).into(),
            token.clone().into(),
        ],
    ))
    .await?;

    Ok(token)
}

/// Unsubscribe info from a token lookup.
#[derive(Debug, Clone)]
pub struct UnsubscribeInfo {
    pub id: Uuid,
    pub recipient_id: Uuid,
    pub event_id: Option<Uuid>,
    pub category: Option<String>,
    pub channel: Option<String>,
}

#[derive(Debug, Clone, FromQueryResult)]
struct UnsubscribeRaw {
    id: String,
    recipient_id: String,
    event_id: Option<String>,
    category: Option<String>,
    channel: Option<String>,
}

/// Look up an unsubscribe record by token.
pub async fn find_by_unsubscribe_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<Option<UnsubscribeInfo>, DbErr> {
    let raw = UnsubscribeRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, recipient_id, event_id, category, channel FROM unsubscribe WHERE token = ?",
        [token.into()],
    ))
    .one(db)
    .await?;

    match raw {
        Some(r) => {
            let event_id = r
                .event_id
                .as_deref()
                .map(Uuid::parse_str)
                .transpose()
                .map_err(|e| DbErr::Custom(format!("invalid event UUID: {e}")))?;

            Ok(Some(UnsubscribeInfo {
                id: Uuid::parse_str(&r.id)
                    .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
                recipient_id: Uuid::parse_str(&r.recipient_id)
                    .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
                event_id,
                category: r.category,
                channel: r.channel,
            }))
        }
        None => Ok(None),
    }
}

/// Apply an unsubscribe token: set the recipient's preference to disabled.
/// If category+channel are specified in the token, disables that specific combo.
/// If only category, disables all channels for that category (no-op for now, just marks the one).
pub async fn apply_unsubscribe(db: &DatabaseConnection, token: &str) -> Result<bool, DbErr> {
    let info = find_by_unsubscribe_token(db, token).await?;
    let info = match info {
        Some(i) => i,
        None => return Ok(false),
    };

    // Determine what to opt out of
    let category = info.category.as_deref().unwrap_or("marketing");
    let channel = info.channel.as_deref().unwrap_or("email");

    set_preference(db, info.recipient_id, category, channel, false).await?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};
    use sea_orm::ConnectionTrait;

    async fn setup() -> (DatabaseConnection, Uuid, Uuid) {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();

        let project_id = Uuid::now_v7();
        let recipient_id = Uuid::now_v7();
        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test')"
        ))
        .await
        .unwrap();
        db.execute_unprepared(&format!(
            "INSERT INTO recipient (id, project_id, external_id) VALUES ('{recipient_id}', '{project_id}', 'user-1')"
        ))
        .await
        .unwrap();

        (db, project_id, recipient_id)
    }

    #[tokio::test]
    async fn set_and_list_preferences() {
        let (db, _, recipient_id) = setup().await;

        // Initially empty
        let prefs = list_preferences(&db, recipient_id).await.unwrap();
        assert!(prefs.is_empty());

        // Set some preferences
        set_preference(&db, recipient_id, "marketing", "email", false)
            .await
            .unwrap();
        set_preference(&db, recipient_id, "marketing", "sms", true)
            .await
            .unwrap();

        let prefs = list_preferences(&db, recipient_id).await.unwrap();
        assert_eq!(prefs.len(), 2);

        // Check opted out
        assert!(is_opted_out(&db, recipient_id, "marketing", "email")
            .await
            .unwrap());
        assert!(!is_opted_out(&db, recipient_id, "marketing", "sms")
            .await
            .unwrap());
        // Non-existent preference means not opted out
        assert!(!is_opted_out(&db, recipient_id, "transactional", "email")
            .await
            .unwrap());

        // Update preference (upsert)
        set_preference(&db, recipient_id, "marketing", "email", true)
            .await
            .unwrap();
        assert!(!is_opted_out(&db, recipient_id, "marketing", "email")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn unsubscribe_token_flow() {
        let (db, _, recipient_id) = setup().await;

        let token = create_unsubscribe_token(
            &db,
            recipient_id,
            None,
            Some("marketing"),
            Some("email"),
        )
        .await
        .unwrap();
        assert!(token.starts_with("unsub_"));

        // Look up token
        let info = find_by_unsubscribe_token(&db, &token)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(info.recipient_id, recipient_id);
        assert_eq!(info.category.as_deref(), Some("marketing"));
        assert_eq!(info.channel.as_deref(), Some("email"));

        // Not opted out yet
        assert!(!is_opted_out(&db, recipient_id, "marketing", "email")
            .await
            .unwrap());

        // Apply unsubscribe
        let applied = apply_unsubscribe(&db, &token).await.unwrap();
        assert!(applied);

        // Now opted out
        assert!(is_opted_out(&db, recipient_id, "marketing", "email")
            .await
            .unwrap());

        // Invalid token returns false
        let applied = apply_unsubscribe(&db, "invalid_token").await.unwrap();
        assert!(!applied);
    }

    #[tokio::test]
    async fn nonexistent_token_returns_none() {
        let (db, _, _) = setup().await;
        let info = find_by_unsubscribe_token(&db, "nonexistent")
            .await
            .unwrap();
        assert!(info.is_none());
    }
}
