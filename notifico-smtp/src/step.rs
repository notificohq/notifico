use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "email.load_template")]
    LoadTemplate { template_id: Uuid },
    #[serde(rename = "email.send")]
    Send { credential: String },
}

pub(crate) const STEPS: &[&str] = &["email.load_template", "email.send"];
