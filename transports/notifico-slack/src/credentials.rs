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
        if value.transport() != Self::TRANSPORT_NAME {
            return Err(EngineError::InvalidCredentialFormat)?;
        }

        match value {
            Credential::Long { value, .. } => {
                Ok(serde_json::from_value(value)
                    .map_err(|_| EngineError::InvalidCredentialFormat)?)
            }
            Credential::Short(url) => Ok(Self {
                token: url.strip_prefix("slack://").unwrap_or_default().to_owned(),
            }),
        }
    }
}

impl TypedCredential for SlackCredentials {
    const TRANSPORT_NAME: &'static str = "slack";
}
