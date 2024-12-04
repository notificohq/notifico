use sea_orm::DbErr;
use std::error::Error;

#[derive(Debug)]
pub enum EngineError {
    InvalidCredentialFormat,
    CredentialNotFound,
    PluginNotFound(String),
    RecipientNotSet,
    ContactNotSet,
    ContactTypeMismatch(String),
    InvalidContactFormat(String),
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
