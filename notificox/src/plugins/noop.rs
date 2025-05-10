use crate::workflow::SerializedNode;
use crate::plugin::{Plugin, NodeType};
use crate::message::Message;

pub struct NoOpPlugin;

impl Plugin for NoOpPlugin {
    fn process_message(&self, _node: &SerializedNode, _message: &mut Message) {
        // Do nothing - this is a no-op plugin
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.noop.v1".to_string(),
            is_trigger: false,
        }]
    }
}