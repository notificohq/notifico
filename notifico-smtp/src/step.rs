use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum CredentialSelector {
    SmtpName { smtp_name: String },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "email.load_template")]
    LoadTemplate { template_id: Uuid },
    #[serde(rename = "email.send")]
    Send(CredentialSelector),
}

pub(crate) const STEPS: &[&str] = &["email.load_template", "email.send"];
