use crate::workflow::SerializedNode;
use crate::plugin::{Plugin, NodeType, Outcome};
use crate::message::Message;

pub struct NoOpPlugin;

impl Plugin for NoOpPlugin {
    fn process_message(&self, _node: &SerializedNode, message: Message, slot: Option<String>) -> Outcome {
        // Do nothing - this is a no-op plugin
        Outcome::Return {
            message,
            slot
        }
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.noop.v1".to_string(),
            is_trigger: false,
        }]
    }
}