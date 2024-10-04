use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "whatsapp.send")]
    Send { credential: String },
}

pub const STEPS: &[&str] = &["whatsapp.load_template", "whatsapp.send"];
