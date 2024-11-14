use notifico_core::error::EngineError;
use sea_orm::DbErr;
use std::io;
use std::io::ErrorKind;

#[derive(Debug)]
pub enum TemplaterError {
    TemplateNotFound,
    Io(io::Error),
    Db(DbErr),
}

impl From<TemplaterError> for EngineError {
    fn from(value: TemplaterError) -> Self {
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
