use notifico_core::pipeline::SerializedStep;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum CredentialSelector {
    SmtpName { smtp_name: String },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum EmailStep {
    #[serde(rename = "email.load_template")]
    LoadTemplate { template_id: Uuid },
    #[serde(rename = "email.send")]
    Send(CredentialSelector),
}

impl TryFrom<SerializedStep> for EmailStep {
    type Error = ();

    fn try_from(value: SerializedStep) -> Result<Self, Self::Error> {
        let s = serde_json::to_string(&value.into_value()).unwrap();

        Ok(serde_json::from_str(&s).unwrap())
    }
}
