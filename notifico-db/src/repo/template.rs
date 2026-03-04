use sea_orm::{DatabaseConnection, DbErr, FromQueryResult, Statement};
use serde_json::Value;
use uuid::Uuid;

/// A fully resolved template row including content body for a specific locale.
#[derive(Debug, Clone)]
pub struct ResolvedTemplate {
    pub template_id: Uuid,
    pub template_name: String,
    pub channel: String,
    pub version: i32,
    pub locale: String,
    pub body: Value,
}

/// Internal raw row returned by the SQL query. UUIDs come back as text from SQLite.
#[derive(Debug, Clone, FromQueryResult)]
struct ResolvedTemplateRaw {
    template_id: String,
    template_name: String,
    channel: String,
    version: i32,
    locale: String,
    body: Value,
}

impl ResolvedTemplateRaw {
    fn into_resolved(self) -> Result<ResolvedTemplate, DbErr> {
        let template_id = Uuid::parse_str(&self.template_id)
            .map_err(|e| DbErr::Custom(format!("invalid template_id UUID: {e}")))?;
        Ok(ResolvedTemplate {
            template_id,
            template_name: self.template_name,
            channel: self.channel,
            version: self.version,
            locale: self.locale,
            body: self.body,
        })
    }
}

/// Resolve a template to its current version content for the given locale,
/// falling back to the default locale if necessary.
///
/// Returns `None` if the template does not exist, has no current version,
/// or has no content in either the requested or default locale.
pub async fn resolve_template(
    db: &DatabaseConnection,
    template_id: Uuid,
    locale: &str,
    default_locale: &str,
) -> Result<Option<ResolvedTemplate>, DbErr> {
    let backend = db.get_database_backend();

    let sql = r#"
        SELECT
            t.id AS template_id,
            t.name AS template_name,
            t.channel,
            tv.version,
            tc.locale,
            tc.body
        FROM template t
        JOIN template_version tv ON tv.template_id = t.id AND tv.is_current = true
        JOIN template_content tc ON tc.template_version_id = tv.id
        WHERE t.id = ? AND tc.locale = ?
        LIMIT 1
    "#;

    // Try exact locale match first.
    let raw = ResolvedTemplateRaw::find_by_statement(Statement::from_sql_and_values(
        backend,
        sql,
        [template_id.to_string().into(), locale.into()],
    ))
    .one(db)
    .await?;

    if let Some(row) = raw {
        return Ok(Some(row.into_resolved()?));
    }

    // Fall back to default locale if it differs from the requested one.
    if locale != default_locale {
        let raw = ResolvedTemplateRaw::find_by_statement(Statement::from_sql_and_values(
            backend,
            sql,
            [template_id.to_string().into(), default_locale.into()],
        ))
        .one(db)
        .await?;

        if let Some(row) = raw {
            return Ok(Some(row.into_resolved()?));
        }
    }

    Ok(None)
}

/// A row from the `pipeline_rule` table.
#[derive(Debug, Clone)]
pub struct PipelineRuleRow {
    pub id: Uuid,
    pub channel: String,
    pub template_id: Uuid,
    pub enabled: bool,
    pub conditions: Option<Value>,
    pub priority: i32,
}

/// Internal raw row for pipeline_rule queries.
#[derive(Debug, Clone, FromQueryResult)]
struct PipelineRuleRaw {
    id: String,
    channel: String,
    template_id: String,
    enabled: bool,
    conditions: Option<Value>,
    priority: i32,
}

impl PipelineRuleRaw {
    fn into_row(self) -> Result<PipelineRuleRow, DbErr> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DbErr::Custom(format!("invalid pipeline_rule id UUID: {e}")))?;
        let template_id = Uuid::parse_str(&self.template_id)
            .map_err(|e| DbErr::Custom(format!("invalid template_id UUID: {e}")))?;
        Ok(PipelineRuleRow {
            id,
            channel: self.channel,
            template_id,
            enabled: self.enabled,
            conditions: self.conditions,
            priority: self.priority,
        })
    }
}

/// Fetch all enabled pipeline rules for the given event, ordered by priority
/// descending (highest priority first).
pub async fn get_pipeline_rules(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> Result<Vec<PipelineRuleRow>, DbErr> {
    let backend = db.get_database_backend();

    let sql = r#"
        SELECT id, channel, template_id, enabled, conditions, priority
        FROM pipeline_rule
        WHERE event_id = ? AND enabled = true
        ORDER BY priority DESC
    "#;

    let rows = PipelineRuleRaw::find_by_statement(Statement::from_sql_and_values(
        backend,
        sql,
        [event_id.to_string().into()],
    ))
    .all(db)
    .await?;

    rows.into_iter().map(|r| r.into_row()).collect()
}

/// A row from the `event` table.
#[derive(Debug, Clone)]
pub struct EventRow {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub category: String,
}

/// Internal raw row for event queries.
#[derive(Debug, Clone, FromQueryResult)]
struct EventRaw {
    id: String,
    project_id: String,
    name: String,
    category: String,
}

impl EventRaw {
    fn into_row(self) -> Result<EventRow, DbErr> {
        let id = Uuid::parse_str(&self.id)
            .map_err(|e| DbErr::Custom(format!("invalid event id UUID: {e}")))?;
        let project_id = Uuid::parse_str(&self.project_id)
            .map_err(|e| DbErr::Custom(format!("invalid project_id UUID: {e}")))?;
        Ok(EventRow {
            id,
            project_id,
            name: self.name,
            category: self.category,
        })
    }
}

/// Find an event by its name within a specific project.
pub async fn find_event_by_name(
    db: &DatabaseConnection,
    project_id: Uuid,
    event_name: &str,
) -> Result<Option<EventRow>, DbErr> {
    let backend = db.get_database_backend();

    let sql = r#"
        SELECT id, project_id, name, category
        FROM event
        WHERE project_id = ? AND name = ?
        LIMIT 1
    "#;

    let raw = EventRaw::find_by_statement(Statement::from_sql_and_values(
        backend,
        sql,
        [project_id.to_string().into(), event_name.into()],
    ))
    .one(db)
    .await?;

    match raw {
        Some(row) => Ok(Some(row.into_row()?)),
        None => Ok(None),
    }
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

    /// Seed a project + template + current version + content rows for multiple locales.
    /// Returns (project_id, template_id, version_id).
    async fn seed_template_with_content(
        db: &DatabaseConnection,
        locales: &[(&str, &str)], // [(locale, body_json), ...]
    ) -> (Uuid, Uuid, Uuid) {
        let project_id = Uuid::now_v7();
        let template_id = Uuid::now_v7();
        let version_id = Uuid::now_v7();

        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test-project')"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO template (id, project_id, name, channel) \
             VALUES ('{template_id}', '{project_id}', 'welcome', 'email')"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO template_version (id, template_id, version, is_current) \
             VALUES ('{version_id}', '{template_id}', 1, true)"
        ))
        .await
        .unwrap();

        for (locale, body) in locales {
            let content_id = Uuid::now_v7();
            db.execute_unprepared(&format!(
                "INSERT INTO template_content (id, template_version_id, locale, body) \
                 VALUES ('{content_id}', '{version_id}', '{locale}', '{body}')"
            ))
            .await
            .unwrap();
        }

        (project_id, template_id, version_id)
    }

    #[tokio::test]
    async fn resolve_template_exact_locale() {
        let db = setup_db().await;
        let (_project_id, template_id, _version_id) = seed_template_with_content(
            &db,
            &[
                ("en", r#"{"subject":"Hello"}"#),
                ("ru", r#"{"subject":"Привет"}"#),
            ],
        )
        .await;

        let result = resolve_template(&db, template_id, "ru", "en")
            .await
            .unwrap();
        let resolved = result.expect("should resolve template");

        assert_eq!(resolved.template_id, template_id);
        assert_eq!(resolved.template_name, "welcome");
        assert_eq!(resolved.channel, "email");
        assert_eq!(resolved.version, 1);
        assert_eq!(resolved.locale, "ru");
        assert_eq!(resolved.body["subject"], "Привет");
    }

    #[tokio::test]
    async fn resolve_template_fallback_locale() {
        let db = setup_db().await;
        let (_project_id, template_id, _version_id) = seed_template_with_content(
            &db,
            &[
                ("en", r#"{"subject":"Hello"}"#),
                ("ru", r#"{"subject":"Привет"}"#),
            ],
        )
        .await;

        // Request "de" which does not exist; should fall back to default "en".
        let result = resolve_template(&db, template_id, "de", "en")
            .await
            .unwrap();
        let resolved = result.expect("should fall back to default locale");

        assert_eq!(resolved.locale, "en");
        assert_eq!(resolved.body["subject"], "Hello");
    }

    #[tokio::test]
    async fn resolve_template_not_found() {
        let db = setup_db().await;

        let random_id = Uuid::now_v7();
        let result = resolve_template(&db, random_id, "en", "en")
            .await
            .unwrap();

        assert!(result.is_none(), "should return None for unknown template");
    }

    #[tokio::test]
    async fn get_pipeline_rules_returns_sorted() {
        let db = setup_db().await;

        let project_id = Uuid::now_v7();
        let event_id = Uuid::now_v7();
        let template_id = Uuid::now_v7();

        db.execute_unprepared(&format!(
            "INSERT INTO project (id, name) VALUES ('{project_id}', 'test-project')"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO template (id, project_id, name, channel) \
             VALUES ('{template_id}', '{project_id}', 'notif', 'sms')"
        ))
        .await
        .unwrap();

        db.execute_unprepared(&format!(
            "INSERT INTO event (id, project_id, name, category) \
             VALUES ('{event_id}', '{project_id}', 'user.signup', 'lifecycle')"
        ))
        .await
        .unwrap();

        let rule_low = Uuid::now_v7();
        let rule_high = Uuid::now_v7();

        // Insert low-priority rule first.
        db.execute_unprepared(&format!(
            "INSERT INTO pipeline_rule (id, event_id, channel, template_id, enabled, priority) \
             VALUES ('{rule_low}', '{event_id}', 'email', '{template_id}', true, 10)"
        ))
        .await
        .unwrap();

        // Insert high-priority rule second.
        db.execute_unprepared(&format!(
            "INSERT INTO pipeline_rule (id, event_id, channel, template_id, enabled, priority) \
             VALUES ('{rule_high}', '{event_id}', 'sms', '{template_id}', true, 50)"
        ))
        .await
        .unwrap();

        // Also insert a disabled rule that should NOT appear.
        let rule_disabled = Uuid::now_v7();
        db.execute_unprepared(&format!(
            "INSERT INTO pipeline_rule (id, event_id, channel, template_id, enabled, priority) \
             VALUES ('{rule_disabled}', '{event_id}', 'push', '{template_id}', false, 100)"
        ))
        .await
        .unwrap();

        let rules = get_pipeline_rules(&db, event_id).await.unwrap();

        assert_eq!(rules.len(), 2, "disabled rule should be excluded");
        // Highest priority first.
        assert_eq!(rules[0].id, rule_high);
        assert_eq!(rules[0].priority, 50);
        assert_eq!(rules[0].channel, "sms");

        assert_eq!(rules[1].id, rule_low);
        assert_eq!(rules[1].priority, 10);
        assert_eq!(rules[1].channel, "email");
    }
}
