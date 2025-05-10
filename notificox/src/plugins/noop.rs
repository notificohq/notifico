use crate::message::Message;
use crate::plugin::{NodeType, Outcome, Plugin};
use crate::workflow::SerializedNode;
use async_trait::async_trait;

pub struct NoOpPlugin;

#[async_trait]
impl Plugin for NoOpPlugin {
    async fn process_message(
        &self,
        _node: &SerializedNode,
        message: Message,
        slot: Option<String>,
    ) -> Outcome {
        // Do nothing - this is a no-op plugin
        Outcome::Return { message, slot }
    }

    fn all_node_types(&self) -> Vec<NodeType> {
        vec![NodeType {
            name: "core.noop.v1".to_string(),
            is_trigger: false,
        }]
    }
}
