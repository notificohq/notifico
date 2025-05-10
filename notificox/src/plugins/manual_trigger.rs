use crate::workflow::SerializedNode;
use crate::plugin::{Plugin, NodeType};
use crate::message::Message;

pub struct ManualTriggerPlugin;

impl Plugin for ManualTriggerPlugin {
    fn process_message(&self, _node: &SerializedNode, _message: &mut Message) {
        // Manual triggers don't need any execution logic
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.trigger.manual.v1".to_string(),
            is_trigger: true,
        }]
    }
} 