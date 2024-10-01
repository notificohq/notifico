use notifico_core::pipeline::SerializedStep;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
pub enum Step {
    #[serde(rename = "sub.check")]
    Check { channel: String },
    #[serde(rename = "sub.list_unsubscribe")]
    ListUnsubscribe,
}

pub(crate) const STEPS: &'static [&'static str] = &["sub.check", "sub.list_unsubscribe"];
