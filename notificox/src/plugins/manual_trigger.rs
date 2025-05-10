use std::borrow::Cow;
use crate::workflow::SerializedNode;
use crate::plugin::Plugin;
use crate::message::Message;

pub struct ManualTriggerPlugin;

impl Plugin for ManualTriggerPlugin {
    fn process_message(&self, _node: &SerializedNode, _message: &mut Message) {
        // Manual triggers don't need any execution logic
    }

    fn is_trigger(&self, node_type: &str) -> bool {
        node_type == "core.trigger.manual.v1"
    }

    fn all_node_types(&self) -> Vec<Cow<'static, str>> {
        vec![Cow::Borrowed("core.trigger.manual.v1")]
    }
} 