use crate::error::TemplaterError;
use crate::source::TemplateSource;
use crate::{PreRenderedTemplate, TemplateSelector};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::info;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(untagged)]
enum PartSelector {
    Inline(String),
    File { file: PathBuf },
}

#[derive(Deserialize)]
struct Descriptor {
    parts: HashMap<String, PartSelector>,
    // attachments: Vec<String>,
    // extras: HashMap<String, String>,
}

pub struct FilesystemSource {
    base_path: PathBuf,
}

impl FilesystemSource {
    pub fn new(base_path: PathBuf) -> Self {
        info!("Template path: {base_path:?}");
        Self { base_path }
    }
}

#[async_trait]
impl TemplateSource for FilesystemSource {
    async fn get_template(
        &self,
        project_id: Uuid,
        template: TemplateSelector,
    ) -> Result<PreRenderedTemplate, TemplaterError> {
        match template {
            TemplateSelector::File { file } => {
                let base_path = if project_id.is_nil() {
                    self.base_path.clone()
                } else {
                    self.base_path.join(project_id.to_string())
                };

                // TODO: `file` can be absolute path, so it can "escape" template directory
                // Ensure, that we are ok with this in server environments.
                let template_path = base_path.join(file);

                let base_path = std::path::absolute(template_path.clone())?
                    .parent()
                    .unwrap()
                    .to_path_buf();

                let content = tokio::fs::read_to_string(template_path).await?;
                let template: Descriptor =
                    toml::from_str(&content).map_err(|_| TemplaterError::InvalidTemplateFormat)?;

                let mut parts = HashMap::new();

                for (name, sel) in template.parts {
                    let content = match sel {
                        PartSelector::Inline(content) => content,
                        PartSelector::File { file } => {
                            // TODO: `file` can be absolute path, so it can "escape" template directory
                            tokio::fs::read_to_string(base_path.join(file)).await?
                        }
                    };
                    parts.insert(name, content);
                }

                Ok(PreRenderedTemplate { parts })
            }
            _ => Err(TemplaterError::TemplateNotFound),
        }
    }
}
