use crate::error::TemplaterError;
use crate::source::{TemplateItem, TemplateSource};
use crate::{entity, PreRenderedTemplate, TemplateSelector};
use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use notifico_core::http::admin::{ListQueryParams, ListableTrait, PaginatedResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use std::collections::HashMap;
use uuid::Uuid;

pub struct DbTemplateSource {
    db: DatabaseConnection,
}

impl DbTemplateSource {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn setup(&self) -> anyhow::Result<()> {
        Ok(Migrator::up(&self.db, None).await?)
    }
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

    async fn get_template_by_id(&self, id: Uuid) -> Result<TemplateItem, TemplaterError> {
        Ok(entity::template::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or(TemplaterError::TemplateNotFound)?
            .into())
    }

    async fn list_templates(
        &self,
        channel: &str,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<TemplateItem>, TemplaterError> {
        Ok(PaginatedResult {
            items: entity::template::Entity::find()
                .apply_params(&params)
                .unwrap()
                .filter(entity::template::Column::Channel.eq(channel))
                .all(&self.db)
                .await?
                .into_iter()
                .map(TemplateItem::from)
                .collect(),
            total_count: entity::template::Entity::find()
                .apply_filter(&params)
                .unwrap()
                .filter(entity::template::Column::Channel.eq(channel))
                .count(&self.db)
                .await?,
        })
    }

    async fn create_template(
        &self,
        mut item: TemplateItem,
    ) -> Result<TemplateItem, TemplaterError> {
        item.id = Uuid::now_v7();
        entity::template::ActiveModel {
            id: Set(item.id),
            project_id: Set(item.project_id),
            name: Set(item.name.clone()),
            channel: Set(item.channel.clone()),
            template: Set(serde_json::to_value(item.template.clone()).unwrap()),
        }
        .insert(&self.db)
        .await?;
        Ok(item)
    }
}

impl From<entity::template::Model> for PreRenderedTemplate {
    fn from(value: entity::template::Model) -> Self {
        PreRenderedTemplate(serde_json::from_value(value.template).unwrap_or(HashMap::new()))
    }
}
