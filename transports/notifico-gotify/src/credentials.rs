use notifico_core::credentials::{RawCredential, TypedCredential};
use notifico_core::error::EngineError;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use url::Url;

#[derive(Serialize, Deserialize)]
pub struct GotifyCredentials {
    pub base_url: Url,
    pub token: String,
}

impl TryFrom<RawCredential> for GotifyCredentials {
    type Error = EngineError;

    fn try_from(value: RawCredential) -> Result<Self, Self::Error> {
        let base_url =
            Url::parse(&value.value).map_err(|_| EngineError::InvalidCredentialFormat)?;

        let query: BTreeMap<Cow<str>, Cow<str>> = base_url.query_pairs().into_iter().collect();
        let token = query
            .get("token")
            .ok_or(EngineError::InvalidCredentialFormat)?
            .to_string();

        let base_url =
            Url::parse(&format!("{}://{}", base_url.scheme(), base_url.authority())).unwrap();

        Ok(Self { base_url, token })
    }
}

impl TypedCredential for GotifyCredentials {
    const TRANSPORT_NAME: &'static str = "gotify";
}
