use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "telegram.load_template")]
    LoadTemplate { template_id: Uuid },
    #[serde(rename = "telegram.send")]
    Send { credential: String },
}

pub(crate) const STEPS: &'static [&'static str] = &["telegram.load_template", "telegram.send"];
