use crate::error::TemplaterError;
use crate::source::TemplateSource;
use crate::{entity, PreRenderedTemplate, TemplateSelector};
use async_trait::async_trait;
use migration::{Migrator, MigratorTrait};
use notifico_core::error::EngineError;
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    Set,
};
use serde::{Deserialize, Serialize};
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
        template: TemplateSelector,
    ) -> Result<PreRenderedTemplate, TemplaterError> {
        Ok(match template {
            TemplateSelector::Name(name) => entity::template::Entity::find()
                .filter(entity::template::Column::ProjectId.eq(project_id))
                .filter(entity::template::Column::Name.eq(name))
                .one(&self.db)
                .await?
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

#[async_trait]
impl AdminCrudTable for DbTemplateSource {
    type Item = TemplateItem;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError> {
        Ok(entity::template::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .map(|m| ItemWithId::from(m).item))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError> {
        let items = entity::template::Entity::find();
        Ok(PaginatedResult {
            items: items
                .clone()
                .apply_params(&params)
                .unwrap()
                .all(&self.db)
                .await?
                .into_iter()
                .map(|m| m.into())
                .collect(),
            total: items.apply_filter(&params).unwrap().count(&self.db).await?,
        })
    }

    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, EngineError> {
        let id = Uuid::now_v7();
        entity::template::ActiveModel {
            id: Set(id),
            project_id: Set(item.project_id),
            name: Set(item.name.clone()),
            channel: Set(item.channel.clone()),
            template: Set(serde_json::from_str(&item.template).unwrap()),
        }
        .insert(&self.db)
        .await?;
        Ok(ItemWithId { id, item })
    }

    async fn update(
        &self,
        id: Uuid,
        item: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, EngineError> {
        entity::template::ActiveModel {
            id: Set(id),
            project_id: Set(item.project_id),
            name: Set(item.name.clone()),
            channel: Set(item.channel.clone()),
            template: Set(serde_json::from_str(&item.template).unwrap()),
        }
        .update(&self.db)
        .await?;
        Ok(ItemWithId { id, item })
    }

    async fn delete(&self, id: Uuid) -> Result<(), EngineError> {
        entity::template::ActiveModel {
            id: Set(id),
            ..Default::default()
        }
        .delete(&self.db)
        .await?;
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TemplateItem {
    pub project_id: Uuid,
    pub channel: String,
    pub name: String,
    pub template: String,
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
            },
        }
    }
}
