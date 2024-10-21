use notifico_core::credentials::TypedCredential;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SmppServerCredentials {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
}

impl TypedCredential for SmppServerCredentials {
    const CREDENTIAL_TYPE: &'static str = "smpp";
}
