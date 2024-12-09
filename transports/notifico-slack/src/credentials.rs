use notifico_core::credentials::{RawCredential, TypedCredential};
use notifico_core::error::EngineError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SlackCredentials {
    pub token: String,
}

impl TryFrom<RawCredential> for SlackCredentials {
    type Error = EngineError;

    fn try_from(value: RawCredential) -> Result<Self, Self::Error> {
        Ok(Self { token: value.value })
    }
}

impl TypedCredential for SlackCredentials {
    const TRANSPORT_NAME: &'static str = "slack";
}
