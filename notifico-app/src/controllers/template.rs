use crate::crud_table::{
    AdminCrudError, AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use crate::entity;
use async_trait::async_trait;
use notifico_template::error::TemplaterError;
use notifico_template::source::TemplateSource;
use notifico_template::{PreRenderedTemplate, TemplateSelector};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

pub struct DbTemplateSource {
    db: DatabaseConnection,
}

impl DbTemplateSource {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}

#[async_trait]
impl TemplateSource for DbTemplateSource {
    async fn get_template(
        &self,
        project_id: Uuid,
        template: TemplateSelector,
    ) -> Result<PreRenderedTemplate, TemplaterError> {
        Ok(match template {
            TemplateSelector::Name(name) => entity::template::Entity::find()
                .filter(entity::template::Column::ProjectId.eq(project_id))
                .filter(entity::template::Column::Name.eq(name))
                .one(&self.db)
                .await
                .map_err(anyhow::Error::new)?
                .ok_or(TemplaterError::TemplateNotFound)?,
            _ => return Err(TemplaterError::TemplateNotFound),
        }
        .into())
    }
}

impl From<entity::template::Model> for PreRenderedTemplate {
    fn from(value: entity::template::Model) -> Self {
        serde_json::from_value(value.template).unwrap_or_default()
    }
}

impl AdminCrudTable for DbTemplateSource {
    type Item = TemplateItem;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, AdminCrudError> {
        Ok(entity::template::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .map(|m| ItemWithId::from(m).item))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, AdminCrudError> {
        let params = params.try_into()?;
        let items = entity::template::Entity::find();
        Ok(PaginatedResult {
            items: items
                .clone()
                .apply_params(&params)?
                .all(&self.db)
                .await?
                .into_iter()
                .map(|m| m.into())
                .collect(),
            total: items.apply_filter(&params)?.count(&self.db).await?,
        })
    }

    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, AdminCrudError> {
        let id = Uuid::now_v7();
        entity::template::ActiveModel {
            id: Set(id),
            project_id: Set(item.project_id),
            name: Set(item.name.clone()),
            channel: Set(item.channel.clone()),
            template: Set(serde_json::from_str(&item.template).unwrap()),
            description: Set(item.description.clone()),
        }
        .insert(&self.db)
        .await?;
        Ok(ItemWithId { id, item })
    }

    async fn update(
        &self,
        id: Uuid,
        item: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, AdminCrudError> {
        entity::template::ActiveModel {
            id: Set(id),
            project_id: Set(item.project_id),
            name: Set(item.name.clone()),
            channel: Set(item.channel.clone()),
            template: Set(serde_json::from_str(&item.template).unwrap()),
            description: Set(item.description.clone()),
        }
        .update(&self.db)
        .await?;
        Ok(ItemWithId { id, item })
    }

    async fn delete(&self, id: Uuid) -> Result<(), AdminCrudError> {
        entity::template::Entity::delete_by_id(id)
            .exec(&self.db)
            .await?;
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateItem {
    pub project_id: Uuid,
    pub channel: String,
    pub name: String,
    pub template: String,
    pub description: String,
}

impl From<entity::template::Model> for ItemWithId<TemplateItem> {
    fn from(value: entity::template::Model) -> Self {
        ItemWithId {
            id: value.id,
            item: TemplateItem {
                project_id: value.project_id,
                template: serde_json::to_string_pretty(&value.template).unwrap(),
                channel: value.channel,
                name: value.name,
                description: value.description,
            },
        }
    }
}
