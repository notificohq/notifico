use crate::error::TemplaterError;
use crate::source::TemplateSource;
use crate::{entity, PreRenderedTemplate, TemplateSelector};
use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;
use uuid::Uuid;

struct DbTemplateSource {
    db: DatabaseConnection,
}

#[async_trait]
impl TemplateSource for DbTemplateSource {
    async fn get_template(
        &self,
        project_id: Uuid,
        channel: &str,
        template: TemplateSelector,
    ) -> Result<PreRenderedTemplate, TemplaterError> {
        Ok(match template {
            TemplateSelector::ByName(name) => entity::template::Entity::find()
                .filter(entity::template::Column::ProjectId.eq(project_id))
                .filter(entity::template::Column::Name.eq(name))
                .filter(entity::template::Column::Channel.eq(channel))
                .one(&self.db)
                .await?
                .ok_or(TemplaterError::TemplateNotFound)?,
        }
        .into())
    }

    async fn get_template_by_id(&self, id: Uuid) -> Result<PreRenderedTemplate, TemplaterError> {
        Ok(entity::template::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(TemplaterError::TemplateNotFound)?
            .into())
    }
}

impl From<entity::template::Model> for PreRenderedTemplate {
    fn from(value: entity::template::Model) -> Self {
        PreRenderedTemplate(serde_json::from_value(value.template).unwrap_or(HashMap::new()))
    }
}
