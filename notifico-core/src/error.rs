use sea_orm::DbErr;
use std::error::Error as StdError;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Invalid credential format")]
    InvalidCredentialFormat,
    #[error("Credential not found")]
    CredentialNotFound,
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
    #[error("Recipient not set")]
    RecipientNotSet,
    #[error("Contact not set")]
    ContactNotSet,
    #[error("Invalid contact type: {0}")]
    ContactTypeMismatch(String),
    #[error("Invalid contact format: {0}")]
    InvalidContactFormat(String),
    #[error("Template rendering error")]
    TemplateRenderingError,
    #[error("Missing template parameter: {0}")]
    MissingTemplateParameter(String),
    #[error("Invalid rendered template format: {0}")]
    InvalidRenderedTemplateFormat(Box<dyn StdError>),
    #[error("Internal error: {0}")]
    InternalError(Box<dyn StdError>),
    #[error("Invalid step: {0}")]
    InvalidStep(serde_json::Error),
    #[error("Missing credential")]
    MissingCredential,
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Message not found: {0}")]
    MessageNotFound(u16),
    #[error("Invalid attachment URL: {0}")]
    InvalidAttachmentUrl(Url),
}

impl From<DbErr> for EngineError {
    fn from(value: DbErr) -> Self {
        Self::InternalError(Box::new(value))
    }
}
