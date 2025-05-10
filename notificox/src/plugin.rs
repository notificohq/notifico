use crate::message::Message;
use crate::workflow::SerializedNode;
use async_trait::async_trait;

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
pub trait Plugin {
    async fn process_message(
        &self,
        node: &SerializedNode,
        message: Message,
        slot: Option<String>,
    ) -> Outcome;
    fn all_node_types(&self) -> Vec<NodeType>;
}
