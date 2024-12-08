use notifico_core::credentials::{Credential, TypedCredential};
use notifico_core::error::EngineError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SlackCredentials {
    pub token: String,
}

impl TryFrom<Credential> for SlackCredentials {
    type Error = EngineError;

    fn try_from(value: Credential) -> Result<Self, Self::Error> {
        Ok(Self { token: value.value })
    }
}

impl TypedCredential for SlackCredentials {
    const TRANSPORT_NAME: &'static str = "slack";
}
