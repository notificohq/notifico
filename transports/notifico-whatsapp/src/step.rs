use notifico_core::credentials::CredentialSelector;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "waba.send")]
    Send { credential: CredentialSelector },
}

pub const STEPS: &[&str] = &["waba.send"];
