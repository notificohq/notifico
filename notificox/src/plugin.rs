use crate::message::Message;
use crate::workflow::SerializedNode;
use async_trait::async_trait;
use uuid::Uuid;

pub struct NodeType {
    pub name: String,
    pub is_trigger: bool,
}

pub enum Outcome {
    Return {
        slot: Option<String>,
        message: Message,
    },
    Error {
        error: String,
    },
}

#[async_trait]
pub trait Plugin: Send + Sync + 'static {
    async fn process_message(
        &self,
        node: &SerializedNode,
        message: Message,
        slot: Option<String>,
    ) -> Outcome;
    fn all_node_types(&self) -> Vec<NodeType>;

    fn register_trigger(&self, node: &SerializedNode, token: u32, workflow_id: Uuid) {}
}
