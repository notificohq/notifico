use std::borrow::Cow;
use crate::workflow::SerializedNode;
use crate::plugin::Plugin;
use crate::message::Message;

pub struct NoOpPlugin;

impl Plugin for NoOpPlugin {
    fn execute_node(&self, _node: &SerializedNode, _message: &mut Message) {
        // Do nothing - this is a no-op plugin
    }

    fn is_trigger(&self, _node_type: &str) -> bool {
        false
    }

    fn all_node_types(&self) -> Vec<Cow<'static, str>> {
        vec![Cow::Borrowed("core.noop.v1")]
    }
}