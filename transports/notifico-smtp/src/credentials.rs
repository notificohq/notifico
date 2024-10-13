use notifico_core::credentials::TypedCredential;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SmtpServerCredentials {
    tls: bool,
    host: String,
    port: Option<u16>,
    username: String,
    password: String,
}

impl TypedCredential for SmtpServerCredentials {
    const CREDENTIAL_TYPE: &'static str = "smtp_server";
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
