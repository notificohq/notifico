use std::sync::Arc;
use crate::workflow::{SerializedNode, ParsedWorkflow};
use crate::plugin_registry::PluginRegistry;
use crate::message::Message;
use tracing;

pub struct WorkflowExecutor {
    plugin_registry: Arc<PluginRegistry>,
}

impl WorkflowExecutor {
    pub fn new(plugin_registry: Arc<PluginRegistry>) -> Self {
        Self { plugin_registry }
    }

    pub fn find_trigger_node<'a>(&self, workflow: &'a ParsedWorkflow) -> Option<&'a SerializedNode> {
        workflow.nodes.values().find(|node| {
            self.plugin_registry.plugins.iter().any(|plugin| plugin.is_trigger(&node.r#type))
        })
    }

    pub fn execute_node(&self, node: &SerializedNode, message: &mut Message) {
        if let Some(plugin) = self.plugin_registry.nodes.get(&node.r#type) {
            tracing::info!("Executing node {} of type {}", node.id, node.r#type);
            plugin.execute_node(node, message);
        } else {
            tracing::warn!("No plugin found for node type: {}", node.r#type);
        }
    }

    pub fn execute_workflow(&self, workflow: &ParsedWorkflow, mut message: Message) {
        // Find the trigger node
        let Some(trigger_node) = self.find_trigger_node(workflow) else {
            tracing::error!("No trigger node found in workflow");
            return;
        };

        // Execute the trigger node
        self.execute_node(trigger_node, &mut message);

        // Execute subsequent nodes in order
        let mut current_node_id = trigger_node.id;
        while let Some(&next_node_id) = workflow.connections.get(&current_node_id) {
            if let Some(next_node) = workflow.nodes.get(&next_node_id) {
                message.node_id = next_node_id;
                self.execute_node(next_node, &mut message);
                current_node_id = next_node_id;
            } else {
                tracing::error!("Node {} not found in workflow", next_node_id);
                break;
            }
        }
    }
} 