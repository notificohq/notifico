pub mod db;
pub mod fs;

use crate::error::TemplaterError;
use crate::{PreRenderedTemplate, TemplateSelector};
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait TemplateSource: Send + Sync + 'static {
    async fn get_template(
        &self,
        project_id: Uuid,
        template: TemplateSelector,
    ) -> Result<PreRenderedTemplate, TemplaterError>;
}
