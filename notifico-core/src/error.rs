use crate::templater::TemplaterError;
use std::borrow::Cow;
use uuid::Uuid;

#[derive(Debug)]
pub enum EngineError {
    InvalidCredentialFormat,
    CredentialNotFound(Cow<'static, str>, String),
    TemplaterError(TemplaterError),
    PluginNotFound(String),
    PipelineInterrupted,
    ContactNotFound(String),
    InvalidContactFormat,
    RecipientNotSet,
    TemplateNotSet,
    ProjectNotFound(Uuid),
}

impl From<TemplaterError> for EngineError {
    fn from(err: TemplaterError) -> Self {
        EngineError::TemplaterError(err)
    }
}
