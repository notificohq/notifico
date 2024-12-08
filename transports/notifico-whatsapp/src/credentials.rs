use notifico_core::credentials::{Credential, TypedCredential};
use notifico_core::error::EngineError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Serialize, Deserialize)]
pub struct WhatsAppCredentials {
    pub(crate) phone_id: u64,
    pub(crate) token: String,
}

impl TryFrom<Credential> for WhatsAppCredentials {
    type Error = EngineError;

    fn try_from(value: Credential) -> Result<Self, Self::Error> {
        static WABA_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = WABA_REGEX.get_or_init(|| Regex::new("^([0-9]+):([0-9a-zA-Z]+)$").unwrap());

        let caps = regex
            .captures(&value.value)
            .ok_or(EngineError::InvalidCredentialFormat)?;

        Ok(Self {
            phone_id: caps[0]
                .parse()
                .map_err(|_| EngineError::InvalidCredentialFormat)?,
            token: caps[1].to_owned(),
        })
    }
}

impl TypedCredential for WhatsAppCredentials {
    const TRANSPORT_NAME: &'static str = "waba";
}
