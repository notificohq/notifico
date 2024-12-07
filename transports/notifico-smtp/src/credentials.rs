use notifico_core::credentials::{Credential, TypedCredential};
use notifico_core::error::EngineError;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::BTreeMap;
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct SmtpServerCredentials {
    tls: bool,
    host: String,
    port: Option<u16>,
    username: String,
    password: String,
}

impl TryFrom<Credential> for SmtpServerCredentials {
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
                let url = url.strip_prefix("smtp:").unwrap_or_default();
                let url = String::from("smtp://") + url;
                let url = Url::parse(&url).map_err(|_| EngineError::InvalidCredentialFormat)?;
                let query: BTreeMap<Cow<str>, Cow<str>> = url.query_pairs().collect();
                let tls = query
                    .get("tls")
                    .map(|v| v.as_ref() == "true")
                    .unwrap_or(false);
                Ok(Self {
                    host: url.host_str().unwrap_or_default().to_owned(),
                    port: url.port(),
                    username: url.username().to_owned(),
                    password: url.password().unwrap_or_default().to_owned(),
                    tls,
                })
            }
        }
    }
}

impl TypedCredential for SmtpServerCredentials {
    const TRANSPORT_NAME: &'static str = "smtp";
}

impl SmtpServerCredentials {
    pub fn into_url(self) -> String {
        let (protocol, port, tls_param) = match self.tls {
            true => ("smtps", 465, "?tls=required"),
            false => ("smtp", 25, ""),
        };

        let port = self.port.unwrap_or(port);

        format!(
            "{protocol}://{}:{}@{}:{port}{tls_param}",
            self.username, self.password, self.host
        )
    }
}
