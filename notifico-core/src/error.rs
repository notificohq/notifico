use std::borrow::Cow;
use std::error::Error;
use uuid::Uuid;

#[derive(Debug)]
pub enum EngineError {
    InvalidCredentialFormat,
    CredentialNotFound(Cow<'static, str>, String),
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
