use crate::workflow::SerializedNode;
use crate::plugin::{Plugin, NodeType, Outcome};
use crate::message::Message;
use tracing;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn process_message(&self, _node: &SerializedNode, message: Message, slot: Option<String>) -> Outcome {
        tracing::info!("Debug Plugin - Message ID: {}, Node ID: {}, Slot: {:?}, Data: {}", 
            message.id, 
            message.node_id,
            slot,
            message.data
        );
        Outcome::Return { 
            message,
            slot
        }
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.debug.v1".to_string(),
            is_trigger: false,
        }]
    }
}