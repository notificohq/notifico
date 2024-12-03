use notifico_core::credentials::CredentialSelector;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "whatsapp.send")]
    Send { credential: CredentialSelector },
}

pub const STEPS: &[&str] = &["whatsapp.send"];
