pub mod db;
pub mod fs;

use crate::error::TemplaterError;
use crate::{entity, PreRenderedTemplate, TemplateSelector};
use async_trait::async_trait;
use notifico_core::http::admin::ItemWithId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
                template: toml::to_string_pretty(&value.template).unwrap(),
                channel: value.channel,
                name: value.name,
            },
        }
    }
}

#[async_trait]
pub trait TemplateSource: Send + Sync + 'static {
    async fn get_template(
        &self,
        project_id: Uuid,
        template: TemplateSelector,
    ) -> Result<PreRenderedTemplate, TemplaterError>;
}
