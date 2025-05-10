use std::borrow::Cow;
use crate::workflow::SerializedNode;
use crate::message::Message;

pub struct NodeType {
    pub name: String,
    pub is_trigger: bool,
}

pub trait Plugin {
    fn process_message(&self, node: &SerializedNode, message: &mut Message);
    fn all_node_types(&self) -> Vec<NodeType>;
} 