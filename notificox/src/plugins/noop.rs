use crate::plugin::{NodeType, Plugin};
use async_trait::async_trait;

pub struct NoOpPlugin;

#[async_trait]
impl Plugin for NoOpPlugin {
    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.noop.v1".to_string(),
            is_trigger: false,
        }]
    }
}
