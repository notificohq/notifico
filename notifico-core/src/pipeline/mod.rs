pub mod event;
pub mod executor;
pub mod storage;

use crate::step::SerializedStep;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Event {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Pipeline {
    pub id: Uuid,
    pub project_id: Uuid,
    pub steps: Vec<SerializedStep>,
}
