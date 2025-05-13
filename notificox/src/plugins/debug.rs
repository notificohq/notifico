use crate::plugin::{NodeEvent, NodeType, Plugin};
use async_trait::async_trait;
use tracing;

pub struct DebugPlugin;

#[async_trait]
impl Plugin for DebugPlugin {
    async fn handle_message(&self, token: u32, event: NodeEvent) {
        tracing::info!(
            "Debug Plugin - Message ID: {}, Slot: {:?}, Data: {:?}",
            event.message.id,
            event.slot,
            event.message.items
        );
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.debug.v1".to_string(),
            is_trigger: false,
        }]
    }
}
