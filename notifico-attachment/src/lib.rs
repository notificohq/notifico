use async_trait::async_trait;
use mime_guess::{mime, Mime};
use notifico_core::engine::{AttachmentMetadata, EnginePlugin, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::step::SerializedStep;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::io;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct AttachmentPlugin {
    allow_file_scheme: bool,
}

impl AttachmentPlugin {
    pub fn new(allow_file_schema: bool) -> Self {
        Self {
            allow_file_scheme: allow_file_schema,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
enum Step {
    #[serde(rename = "attachment.attach")]
    Attach {
        #[serde(default)]
        message: u16,
        attachments: Vec<AttachmentMetadata>,
    },
}

#[async_trait]
impl EnginePlugin for AttachmentPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.convert_step()?;

        match step {
            Step::Attach {
                message,
                attachments,
            } => {
                for info in attachments {
                    if info.url.scheme() == "file" && !self.allow_file_scheme {
                        return Err(EngineError::InvalidAttachmentUrl(info.url));
                    }

                    let Some(message) = context.messages.get_mut(message as usize) else {
                        return Err(EngineError::MessageNotFound(message));
                    };
                    message.attachments.push(info);
                }

                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        vec!["attachment.attach".into()]
    }
}

pub struct AttachedFile {
    pub file: File,
    pub file_name: String,
    pub mime_type: Mime,
    pub size: u64,
    pub extras: HashMap<String, String>,
}

impl AttachedFile {
    pub async fn content(&mut self) -> Result<Vec<u8>, io::Error> {
        let mut content = Vec::new();
        self.file.read_to_end(&mut content).await?;
        Ok(content)
    }
}

impl AttachmentPlugin {
    pub async fn get_attachment(
        &self,
        info: &AttachmentMetadata,
    ) -> Result<AttachedFile, EngineError> {
        if info.url.scheme() == "file" && self.allow_file_scheme {
            let Ok(file_path) = info.url.to_file_path() else {
                return Err(EngineError::InvalidAttachmentUrl(info.url.clone()));
            };

            let file = File::open(file_path.clone()).await?;
            let file_name = match (info.file_name.clone(), file_path.file_name()) {
                (Some(file_name), _) => file_name,
                (None, Some(file_name)) => file_name.to_string_lossy().to_string(),
                (None, None) => "attachment.bin".to_string(),
            };

            let guessed_mimes = mime_guess::from_path(&file_name);
            let mime_type = guessed_mimes.first_or(mime::APPLICATION_OCTET_STREAM);

            let size = file.metadata().await?.len();

            return Ok(AttachedFile {
                file,
                file_name,
                mime_type,
                size,
                extras: info.extras.clone(),
            });
        }
        unimplemented!()
    }

    pub async fn get_attachments(
        &self,
        info: &[AttachmentMetadata],
    ) -> Result<Vec<AttachedFile>, EngineError> {
        // todo: use joinset and run in parallel
        let mut attachments = Vec::new();
        for info in info {
            attachments.push(self.get_attachment(info).await?);
        }
        Ok(attachments)
    }
}
