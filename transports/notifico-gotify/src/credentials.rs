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
        let url = Url::parse(&value.value).map_err(|_| EngineError::InvalidCredentialFormat)?;

        Ok(Self { url })
    }
}

impl TypedCredential for GotifyCredentials {
    const TRANSPORT_NAME: &'static str = "gotify";
}
