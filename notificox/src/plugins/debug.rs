use std::borrow::Cow;
use crate::workflow::SerializedNode;
use crate::plugin::Plugin;
use crate::message::Message;
use tracing;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn process_message(&self, _node: &SerializedNode, message: &mut Message) {
        tracing::info!("Debug Plugin - Message ID: {}, Node ID: {}, Data: {}", 
            message.id, 
            message.node_id, 
            message.data
        );
    }

    fn is_trigger(&self, _node_type: &str) -> bool {
        false
    }

    fn all_node_types(&self) -> Vec<Cow<'static, str>> {
        vec![Cow::Borrowed("core.debug.v1")]
    }
}