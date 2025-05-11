use crate::message::Message;
use crate::plugin::{NodeType, Outcome, Plugin};
use crate::workflow::SerializedNode;
use async_trait::async_trait;

pub struct ManualTriggerPlugin;

#[async_trait]
impl Plugin for ManualTriggerPlugin {
    async fn process_message(
        &self,
        _node: &SerializedNode,
        message: Message,
        _slot: Option<String>,
    ) -> Outcome {
        // Manual triggers don't need any execution logic
        Outcome::Return {
            message,
            slot: None,
        }
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.trigger.manual.v1".to_string(),
            is_trigger: true,
        }]
    }
}
