use crate::workflow::SerializedNode;
use crate::plugin::{Plugin, NodeType};
use crate::message::Message;
use tracing;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn process_message(&self, _node: &SerializedNode, message: &mut Message) {
        tracing::info!("Debug Plugin - Message ID: {}, Node ID: {}, Data: {}", 
            message.id, 
            message.node_id, 
            message.data
        );
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.debug.v1".to_string(),
            is_trigger: false,
        }]
    }
}