use crate::error::TemplaterError;
use crate::{PreRenderedTemplate, TemplateSelector};
use async_trait::async_trait;
use sea_orm::prelude::Uuid;

pub mod local;

#[async_trait]
pub trait TemplateSource: Send + Sync + 'static {
    async fn get_template(
        &self,
        project_id: Uuid,
        channel: &str,
        template: TemplateSelector,
    ) -> Result<PreRenderedTemplate, TemplaterError>;

    async fn get_template_by_id(&self, id: Uuid) -> Result<PreRenderedTemplate, TemplaterError>;
}
