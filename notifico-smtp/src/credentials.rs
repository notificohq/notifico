use notifico_core::error::EngineError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct SmtpServerCredentials {
    tls: bool,
    host: String,
    port: Option<u16>,
    username: String,
    password: String,
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

impl TryFrom<Value> for SmtpServerCredentials {
    type Error = EngineError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value).map_err(|_| EngineError::InvalidCredentialFormat)
    }
}
