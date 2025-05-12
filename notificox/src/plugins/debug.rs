use crate::message::Message;
use crate::plugin::{NodeType, Outcome, Plugin};
use crate::workflow::SerializedNode;
use async_trait::async_trait;
use tracing;

pub struct DebugPlugin;

#[async_trait]
impl Plugin for DebugPlugin {
    async fn process_message(
        &self,
        node: &SerializedNode,
        message: Message,
        slot: Option<String>,
    ) -> Outcome {
        tracing::info!(
            "Debug Plugin - Message ID: {}, Node ID: {}, Slot: {:?}, Data: {:?}",
            message.id,
            node.id,
            slot,
            message.items
        );
        Outcome::Return {
            message,
            slot: None,
        }
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.debug.v1".to_string(),
            is_trigger: false,
        }]
    }
}
