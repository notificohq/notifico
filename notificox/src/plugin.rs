use std::borrow::Cow;
use crate::workflow::SerializedNode;
use crate::message::Message;

pub struct NodeType {
    pub name: String,
    pub is_trigger: bool,
}

pub enum Outcome {
    Return { slot: Option<String>, message: Message },
    Error { message: Message, error: String },
}

pub trait Plugin {
    fn process_message(&self, node: &SerializedNode, message: Message, slot: Option<String>) -> Outcome;
    fn all_node_types(&self) -> Vec<NodeType>;
} 