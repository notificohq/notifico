use crate::message::Message;
use crate::schemas::{NodeId, SerializedNode};
use async_trait::async_trait;
use uuid::Uuid;

pub struct NodeType {
    pub name: String,
    pub is_trigger: bool,
}

pub struct NodeEvent {
    pub token: u32,
    pub message: Message,
    pub slot: Option<String>,
}

pub struct NodeStaticContext {
    pub node_id: NodeId,
    pub workflow_id: Uuid,
}

#[async_trait]
pub trait Plugin: Send + Sync + 'static {
    fn all_node_types(&self) -> Vec<NodeType>;

    fn run_node(&self, token: u32, node: &SerializedNode, context: NodeStaticContext) {}
    fn shutdown_node(&self, token: u32) {}

    async fn handle_message(&self, token: u32, event: NodeEvent) {}
}
