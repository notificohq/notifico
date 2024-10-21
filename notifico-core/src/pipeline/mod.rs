pub mod runner;
pub mod storage;

use crate::step::SerializedStep;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Pipeline {
    pub channel: String,
    pub events: Vec<String>,
    pub steps: Vec<SerializedStep>,
}

// Process Request

// Runner
