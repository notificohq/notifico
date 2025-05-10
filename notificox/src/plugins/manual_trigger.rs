use crate::workflow::SerializedNode;
use crate::plugin::{Plugin, NodeType, Outcome};
use crate::message::Message;

pub struct ManualTriggerPlugin;

impl Plugin for ManualTriggerPlugin {
    fn process_message(&self, _node: &SerializedNode, message: Message, slot: Option<String>) -> Outcome {
        // Manual triggers don't need any execution logic
        Outcome::Return {
            message,
            slot
        }
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.trigger.manual.v1".to_string(),
            is_trigger: true,
        }]
    }
} 