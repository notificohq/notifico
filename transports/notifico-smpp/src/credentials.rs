use notifico_core::credentials::{Credential, TypedCredential};
use notifico_core::error::EngineError;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct SmppServerCredentials {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

impl TryFrom<Credential> for SmppServerCredentials {
    type Error = EngineError;

    fn try_from(value: Credential) -> Result<Self, Self::Error> {
        let url = String::from("smpp://") + &value.value;
        let url = Url::parse(&url).map_err(|_| EngineError::InvalidCredentialFormat)?;
        Ok(Self {
            host: url.host_str().unwrap_or_default().to_owned(),
            port: url.port().unwrap_or_default(),
            username: url.username().to_owned(),
            password: url.password().unwrap_or_default().to_owned(),
        })
    }
}

impl TypedCredential for SmppServerCredentials {
    const TRANSPORT_NAME: &'static str = "smpp";
}
