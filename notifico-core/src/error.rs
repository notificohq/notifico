use crate::templater::TemplaterError;
use std::borrow::Cow;
use std::error::Error;
use uuid::Uuid;

#[derive(Debug)]
pub enum EngineError {
    InvalidCredentialFormat,
    CredentialNotFound(Cow<'static, str>, String),
    TemplaterError(TemplaterError),
    PluginNotFound(String),
    ContactNotFound(String),
    InvalidContactFormat,
    RecipientNotSet,
    TemplateNotSet,
    ProjectNotFound(Uuid),
    TemplateRenderingError,
    InternalError(Box<dyn Error>),
    InvalidStep(serde_json::Error),
}

impl From<TemplaterError> for EngineError {
    fn from(err: TemplaterError) -> Self {
        EngineError::TemplaterError(err)
    }
}
