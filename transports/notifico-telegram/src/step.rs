use notifico_core::credentials::CredentialSelector;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "telegram.send")]
    Send { credential: CredentialSelector },
}

pub(crate) const STEPS: &[&str] = &["telegram.send"];
