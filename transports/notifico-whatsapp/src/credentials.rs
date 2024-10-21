use notifico_core::credentials::TypedCredential;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WhatsAppCredentials {
    pub(crate) phone_id: u64,
    pub(crate) token: String,
}

impl TypedCredential for WhatsAppCredentials {
    const CREDENTIAL_TYPE: &'static str = "whatsapp_business";
}
