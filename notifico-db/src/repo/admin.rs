use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, FromQueryResult, Statement};
use serde_json::Value;
use uuid::Uuid;

// ── Project ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProjectRow {
    pub id: Uuid,
    pub name: String,
    pub default_locale: String,
    pub settings: Value,
}

#[derive(Debug, Clone, FromQueryResult)]
struct ProjectRaw {
    id: String,
    name: String,
    default_locale: String,
    settings: String,
}

impl ProjectRaw {
    fn into_row(self) -> Result<ProjectRow, DbErr> {
        Ok(ProjectRow {
            id: Uuid::parse_str(&self.id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            name: self.name,
            default_locale: self.default_locale,
            settings: serde_json::from_str(&self.settings)
                .unwrap_or(Value::Object(Default::default())),
        })
    }
}

pub async fn list_projects(db: &DatabaseConnection) -> Result<Vec<ProjectRow>, DbErr> {
    let rows = ProjectRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, name, default_locale, settings FROM project ORDER BY name",
        [],
    ))
    .all(db)
    .await?;
    rows.into_iter().map(|r| r.into_row()).collect()
}

pub async fn get_project(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<ProjectRow>, DbErr> {
    let raw = ProjectRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, name, default_locale, settings FROM project WHERE id = ?",
        [id.to_string().into()],
    ))
    .one(db)
    .await?;
    match raw {
        Some(r) => Ok(Some(r.into_row()?)),
        None => Ok(None),
    }
}

pub async fn create_project(
    db: &DatabaseConnection,
    id: Uuid,
    name: &str,
    default_locale: &str,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO project (id, name, default_locale) VALUES (?, ?, ?)",
        [id.to_string().into(), name.into(), default_locale.into()],
    ))
    .await?;
    Ok(())
}

pub async fn update_project(
    db: &DatabaseConnection,
    id: Uuid,
    name: &str,
    default_locale: &str,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE project SET name = ?, default_locale = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        [name.into(), default_locale.into(), id.to_string().into()],
    ))
    .await?;
    Ok(())
}

pub async fn delete_project(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM project WHERE id = ?",
        [id.to_string().into()],
    ))
    .await?;
    Ok(())
}

// ── Event ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct EventRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub category: String,
    pub description: String,
}

#[derive(Debug, Clone, FromQueryResult)]
struct EventRaw {
    id: String,
    project_id: String,
    name: String,
    category: String,
    description: String,
}

impl EventRaw {
    fn into_row(self) -> Result<EventRow, DbErr> {
        Ok(EventRow {
            id: Uuid::parse_str(&self.id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            project_id: Uuid::parse_str(&self.project_id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            name: self.name,
            category: self.category,
            description: self.description,
        })
    }
}

pub async fn list_events(
    db: &DatabaseConnection,
    project_id: Uuid,
) -> Result<Vec<EventRow>, DbErr> {
    let rows = EventRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, project_id, name, category, description FROM event WHERE project_id = ? ORDER BY name",
        [project_id.to_string().into()],
    ))
    .all(db)
    .await?;
    rows.into_iter().map(|r| r.into_row()).collect()
}

pub async fn get_event(db: &DatabaseConnection, id: Uuid) -> Result<Option<EventRow>, DbErr> {
    let raw = EventRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, project_id, name, category, description FROM event WHERE id = ?",
        [id.to_string().into()],
    ))
    .one(db)
    .await?;
    match raw {
        Some(r) => Ok(Some(r.into_row()?)),
        None => Ok(None),
    }
}

pub async fn create_event(
    db: &DatabaseConnection,
    id: Uuid,
    project_id: Uuid,
    name: &str,
    category: &str,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO event (id, project_id, name, category) VALUES (?, ?, ?, ?)",
        [
            id.to_string().into(),
            project_id.to_string().into(),
            name.into(),
            category.into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn update_event(
    db: &DatabaseConnection,
    id: Uuid,
    name: &str,
    category: &str,
    description: &str,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE event SET name = ?, category = ?, description = ? WHERE id = ?",
        [
            name.into(),
            category.into(),
            description.into(),
            id.to_string().into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn delete_event(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM event WHERE id = ?",
        [id.to_string().into()],
    ))
    .await?;
    Ok(())
}

// ── Pipeline Rule ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RuleRow {
    pub id: Uuid,
    pub event_id: Uuid,
    pub channel: String,
    pub template_id: Uuid,
    pub enabled: bool,
    pub priority: i32,
}

#[derive(Debug, Clone, FromQueryResult)]
struct RuleRaw {
    id: String,
    event_id: String,
    channel: String,
    template_id: String,
    enabled: bool,
    priority: i32,
}

impl RuleRaw {
    fn into_row(self) -> Result<RuleRow, DbErr> {
        Ok(RuleRow {
            id: Uuid::parse_str(&self.id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            event_id: Uuid::parse_str(&self.event_id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            channel: self.channel,
            template_id: Uuid::parse_str(&self.template_id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            enabled: self.enabled,
            priority: self.priority,
        })
    }
}

pub async fn list_rules(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> Result<Vec<RuleRow>, DbErr> {
    let rows = RuleRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, event_id, channel, template_id, enabled, priority FROM pipeline_rule WHERE event_id = ? ORDER BY priority DESC",
        [event_id.to_string().into()],
    ))
    .all(db)
    .await?;
    rows.into_iter().map(|r| r.into_row()).collect()
}

pub async fn create_rule(
    db: &DatabaseConnection,
    id: Uuid,
    event_id: Uuid,
    channel: &str,
    template_id: Uuid,
    priority: i32,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO pipeline_rule (id, event_id, channel, template_id, priority) VALUES (?, ?, ?, ?, ?)",
        [
            id.to_string().into(),
            event_id.to_string().into(),
            channel.into(),
            template_id.to_string().into(),
            priority.into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn update_rule(
    db: &DatabaseConnection,
    id: Uuid,
    channel: &str,
    template_id: Uuid,
    enabled: bool,
    priority: i32,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "UPDATE pipeline_rule SET channel = ?, template_id = ?, enabled = ?, priority = ? WHERE id = ?",
        [
            channel.into(),
            template_id.to_string().into(),
            enabled.into(),
            priority.into(),
            id.to_string().into(),
        ],
    ))
    .await?;
    Ok(())
}

pub async fn delete_rule(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM pipeline_rule WHERE id = ?",
        [id.to_string().into()],
    ))
    .await?;
    Ok(())
}

// ── Template ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TemplateRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub channel: String,
}

#[derive(Debug, Clone, FromQueryResult)]
struct TemplateRaw {
    id: String,
    project_id: String,
    name: String,
    channel: String,
}

impl TemplateRaw {
    fn into_row(self) -> Result<TemplateRow, DbErr> {
        Ok(TemplateRow {
            id: Uuid::parse_str(&self.id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            project_id: Uuid::parse_str(&self.project_id)
                .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
            name: self.name,
            channel: self.channel,
        })
    }
}

pub async fn list_templates(
    db: &DatabaseConnection,
    project_id: Uuid,
) -> Result<Vec<TemplateRow>, DbErr> {
    let rows = TemplateRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, project_id, name, channel FROM template WHERE project_id = ? ORDER BY name",
        [project_id.to_string().into()],
    ))
    .all(db)
    .await?;
    rows.into_iter().map(|r| r.into_row()).collect()
}

pub async fn get_template(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<TemplateRow>, DbErr> {
    let raw = TemplateRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, project_id, name, channel FROM template WHERE id = ?",
        [id.to_string().into()],
    ))
    .one(db)
    .await?;
    match raw {
        Some(r) => Ok(Some(r.into_row()?)),
        None => Ok(None),
    }
}

pub async fn create_template(
    db: &DatabaseConnection,
    id: Uuid,
    project_id: Uuid,
    name: &str,
    channel: &str,
) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO template (id, project_id, name, channel) VALUES (?, ?, ?, ?)",
        [
            id.to_string().into(),
            project_id.to_string().into(),
            name.into(),
            channel.into(),
        ],
    ))
    .await?;

    // Create initial version (v1, current)
    let version_id = Uuid::now_v7();
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO template_version (id, template_id, version, is_current) VALUES (?, ?, 1, true)",
        [version_id.to_string().into(), id.to_string().into()],
    ))
    .await?;

    Ok(())
}

pub async fn delete_template(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM template WHERE id = ?",
        [id.to_string().into()],
    ))
    .await?;
    Ok(())
}

/// Set template content for a locale on the current version.
pub async fn set_template_content(
    db: &DatabaseConnection,
    template_id: Uuid,
    locale: &str,
    body: &Value,
) -> Result<(), DbErr> {
    #[derive(Debug, FromQueryResult)]
    struct VersionId {
        id: String,
    }

    let version = VersionId::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM template_version WHERE template_id = ? AND is_current = true",
        [template_id.to_string().into()],
    ))
    .one(db)
    .await?
    .ok_or_else(|| DbErr::Custom("No current version found".into()))?;

    let body_json =
        serde_json::to_string(body).map_err(|e| DbErr::Custom(format!("JSON error: {e}")))?;

    #[derive(Debug, FromQueryResult)]
    struct ContentExists {
        id: String,
    }

    let existing = ContentExists::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id FROM template_content WHERE template_version_id = ? AND locale = ?",
        [version.id.clone().into(), locale.into()],
    ))
    .one(db)
    .await?;

    if let Some(row) = existing {
        db.execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE template_content SET body = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            [body_json.into(), row.id.into()],
        ))
        .await?;
    } else {
        let content_id = Uuid::now_v7();
        db.execute_raw(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT INTO template_content (id, template_version_id, locale, body) VALUES (?, ?, ?, ?)",
            [
                content_id.to_string().into(),
                version.id.into(),
                locale.into(),
                body_json.into(),
            ],
        ))
        .await?;
    }

    Ok(())
}

// ── Credential summary ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CredentialSummary {
    pub id: Uuid,
    pub name: String,
    pub channel: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, FromQueryResult)]
struct CredentialSummaryRaw {
    id: String,
    name: String,
    channel: String,
    enabled: bool,
}

pub async fn list_credentials(
    db: &DatabaseConnection,
    project_id: Uuid,
) -> Result<Vec<CredentialSummary>, DbErr> {
    let rows = CredentialSummaryRaw::find_by_statement(Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT id, name, channel, enabled FROM credential WHERE project_id = ? ORDER BY name",
        [project_id.to_string().into()],
    ))
    .all(db)
    .await?;

    rows.into_iter()
        .map(|r| {
            Ok(CredentialSummary {
                id: Uuid::parse_str(&r.id)
                    .map_err(|e| DbErr::Custom(format!("invalid UUID: {e}")))?,
                name: r.name,
                channel: r.channel,
                enabled: r.enabled,
            })
        })
        .collect()
}

pub async fn delete_credential(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    db.execute_raw(Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM credential WHERE id = ?",
        [id.to_string().into()],
    ))
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connect, run_migrations};
    use serde_json::json;

    async fn setup() -> DatabaseConnection {
        let db = connect("sqlite::memory:").await.unwrap();
        run_migrations(&db).await.unwrap();
        db
    }

    #[tokio::test]
    async fn crud_project() {
        let db = setup().await;
        let id = Uuid::now_v7();

        create_project(&db, id, "Test Project", "en").await.unwrap();

        let project = get_project(&db, id).await.unwrap().unwrap();
        assert_eq!(project.name, "Test Project");
        assert_eq!(project.default_locale, "en");

        let projects = list_projects(&db).await.unwrap();
        assert_eq!(projects.len(), 1);

        update_project(&db, id, "Updated", "ru").await.unwrap();
        let updated = get_project(&db, id).await.unwrap().unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.default_locale, "ru");

        delete_project(&db, id).await.unwrap();
        assert!(get_project(&db, id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn crud_event() {
        let db = setup().await;
        let project_id = Uuid::now_v7();
        create_project(&db, project_id, "P1", "en").await.unwrap();

        let event_id = Uuid::now_v7();
        create_event(&db, event_id, project_id, "order.confirmed", "transactional")
            .await
            .unwrap();

        let events = list_events(&db, project_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].name, "order.confirmed");

        update_event(
            &db,
            event_id,
            "order.shipped",
            "marketing",
            "Shipping notification",
        )
        .await
        .unwrap();
        let updated = get_event(&db, event_id).await.unwrap().unwrap();
        assert_eq!(updated.name, "order.shipped");
        assert_eq!(updated.description, "Shipping notification");

        delete_event(&db, event_id).await.unwrap();
        assert!(get_event(&db, event_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn crud_rule() {
        let db = setup().await;
        let project_id = Uuid::now_v7();
        create_project(&db, project_id, "P1", "en").await.unwrap();

        let event_id = Uuid::now_v7();
        create_event(&db, event_id, project_id, "test", "transactional")
            .await
            .unwrap();

        let template_id = Uuid::now_v7();
        create_template(&db, template_id, project_id, "Welcome", "email")
            .await
            .unwrap();

        let rule_id = Uuid::now_v7();
        create_rule(&db, rule_id, event_id, "email", template_id, 10)
            .await
            .unwrap();

        let rules = list_rules(&db, event_id).await.unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].channel, "email");
        assert_eq!(rules[0].priority, 10);

        update_rule(&db, rule_id, "sms", template_id, false, 5)
            .await
            .unwrap();
        let updated = list_rules(&db, event_id).await.unwrap();
        assert_eq!(updated[0].channel, "sms");
        assert!(!updated[0].enabled);

        delete_rule(&db, rule_id).await.unwrap();
        assert!(list_rules(&db, event_id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn crud_template_with_content() {
        let db = setup().await;
        let project_id = Uuid::now_v7();
        create_project(&db, project_id, "P1", "en").await.unwrap();

        let template_id = Uuid::now_v7();
        create_template(&db, template_id, project_id, "Welcome Email", "email")
            .await
            .unwrap();

        let templates = list_templates(&db, project_id).await.unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "Welcome Email");

        let body = json!({"subject": "Welcome {{ name }}", "text": "Hello {{ name }}"});
        set_template_content(&db, template_id, "en", &body)
            .await
            .unwrap();

        // Verify via existing resolve_template
        let resolved =
            crate::repo::template::resolve_template(&db, template_id, "en", "en")
                .await
                .unwrap()
                .unwrap();
        assert_eq!(resolved.body["subject"], "Welcome {{ name }}");

        // Update content (idempotent)
        let body2 = json!({"subject": "Hi {{ name }}", "text": "Updated"});
        set_template_content(&db, template_id, "en", &body2)
            .await
            .unwrap();
        let resolved2 =
            crate::repo::template::resolve_template(&db, template_id, "en", "en")
                .await
                .unwrap()
                .unwrap();
        assert_eq!(resolved2.body["subject"], "Hi {{ name }}");

        delete_template(&db, template_id).await.unwrap();
        assert!(get_template(&db, template_id).await.unwrap().is_none());
    }
}
