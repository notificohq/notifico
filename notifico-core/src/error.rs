use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Template rendering error: {0}")]
    TemplateRender(String),

    #[error("Recipient not found: {0}")]
    RecipientNotFound(String),

    #[error("Channel not registered: {0}")]
    ChannelNotRegistered(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
