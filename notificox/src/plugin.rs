use std::borrow::Cow;
use crate::workflow::SerializedNode;
use crate::message::Message;

pub trait Plugin {
    fn process_message(&self, node: &SerializedNode, message: &mut Message);
    fn is_trigger(&self, node_type: &str) -> bool;
    fn all_node_types(&self) -> Vec<Cow<'static, str>>;
} 