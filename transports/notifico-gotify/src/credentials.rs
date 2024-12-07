use notifico_core::credentials::{Credential, TypedCredential};
use notifico_core::error::EngineError;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct GotifyCredentials {
    pub url: Url,
}

impl TryFrom<Credential> for GotifyCredentials {
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
            Credential::Short(url) => {
                let mut url = url.splitn(2, "://");
                let _ = url.next();
                let url = url.next().unwrap_or_default();
                let url = Url::parse(url).map_err(|_| EngineError::InvalidCredentialFormat)?;

                Ok(Self { url })
            }
        }
    }
}

impl TypedCredential for GotifyCredentials {
    const TRANSPORT_NAME: &'static str = "gotify";
}
