use notifico_core::error::EngineError;
use sea_orm::DbErr;
use std::io;
use std::io::ErrorKind;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TemplaterError {
    #[error("Template not found")]
    TemplateNotFound,
    #[error("I/O error: {0}")]
    Io(io::Error),
    #[error("SeaORM error: {0}")]
    Db(DbErr),
    #[error("Jinja error: {0}")]
    JinjaError(#[from] minijinja::Error),
}

impl From<TemplaterError> for EngineError {
    fn from(_value: TemplaterError) -> Self {
        EngineError::TemplateRenderingError
    }
}

impl From<io::Error> for TemplaterError {
    fn from(value: io::Error) -> Self {
        match value.kind() {
            ErrorKind::NotFound => Self::TemplateNotFound,
            _ => Self::Io(value),
        }
    }
}

impl From<DbErr> for TemplaterError {
    fn from(value: DbErr) -> Self {
        TemplaterError::Db(value)
    }
}
