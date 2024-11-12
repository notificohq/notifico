use crate::error::TemplaterError;
use crate::source::local::descriptor::{Descriptor, TemplateSourceSelector};
use crate::source::TemplateSource;
use crate::{PreRenderedTemplate, TemplateSelector};
use async_trait::async_trait;
use sea_orm::prelude::Uuid;
use std::path::{Path, PathBuf};

mod descriptor;

pub struct LocalTemplateSource {
    basepath: PathBuf,
}

impl LocalTemplateSource {
    pub fn new(basepath: &Path) -> Self {
        Self {
            basepath: basepath.into(),
        }
    }
}

#[async_trait]
impl TemplateSource for LocalTemplateSource {
    async fn get_template(
        &self,
        project_id: Uuid,
        channel: &str,
        template: TemplateSelector,
    ) -> Result<PreRenderedTemplate, TemplaterError> {
        let template_dir = self.basepath.join(channel);
        let descriptor_path = match template {
            TemplateSelector::ByName(name) => template_dir.join(name),
        };

        // Read the template descriptor file
        let descriptor = tokio::fs::read_to_string(descriptor_path).await?;

        // Parse the template descriptor JSON into a map
        let descriptor: Descriptor = toml::from_str(&descriptor).unwrap();

        // Initialize an empty map to store the rendered template data
        let mut template = PreRenderedTemplate::default();

        // Iterate over the parts in the template descriptor
        for (part, selector) in descriptor.template {
            // Deserialize the template source selector
            let selector: TemplateSourceSelector = selector.try_into().unwrap();

            // Determine the template source based on the selector
            let content = match selector {
                TemplateSourceSelector::Content(content) => content,
                TemplateSourceSelector::File(file) => {
                    // Read the template content from a file
                    tokio::fs::read_to_string(template_dir.join(file)).await?
                }
            };

            // Insert the rendered template part into the data map
            template.0.insert(part, content);
        }

        Ok(template)
    }
}
