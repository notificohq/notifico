use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("Invalid credential format")]
    InvalidCredentialFormat,
    #[error("Credential not found")]
    CredentialNotFound,
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),
    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),
    #[error("Invalid step: {0}")]
    InvalidStep(serde_json::Error),
    #[error("Missing credential")]
    MissingCredential,
}
