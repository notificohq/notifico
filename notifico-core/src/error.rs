use sea_orm::DbErr;
use std::error::Error;
use uuid::Uuid;

#[derive(Debug)]
pub enum EngineError {
    InvalidCredentialFormat,
    CredentialNotFound,
    PluginNotFound(String),
    ContactNotFound(String),
    InvalidContactFormat,
    RecipientNotSet,
    ProjectNotFound(Uuid),
    TemplateRenderingError,
    MissingTemplateParameter(String),
    InvalidRenderedTemplateFormat(Box<dyn Error>),
    InternalError(Box<dyn Error>),
    InvalidStep(serde_json::Error),
}

impl From<DbErr> for EngineError {
    fn from(value: DbErr) -> Self {
        Self::InternalError(Box::new(value))
    }
}

impl From<Box<dyn Error>> for EngineError {
    fn from(value: Box<dyn Error>) -> Self {
        Self::InternalError(value)
    }
}
