use std::sync::Arc;
use crate::workflow::{SerializedNode, ParsedWorkflow, NodeId};
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
            self.plugin_registry.triggers.contains(&node.r#type)
        })
    }

    pub fn process_message(&self, node: &SerializedNode, message: &mut Message) {
        if let Some(plugin) = self.plugin_registry.nodes.get(&node.r#type) {
            tracing::info!("Executing node {} of type {}", node.id, node.r#type);
            plugin.process_message(node, message);
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
        self.process_message(trigger_node, &mut message);

        // Execute all connected nodes recursively
        self.execute_connected_nodes(workflow, &mut message, trigger_node.id);
    }

    fn execute_connected_nodes(&self, workflow: &ParsedWorkflow, message: &mut Message, current_node_id: NodeId) {
        if let Some(target_nodes) = workflow.connections.get(&current_node_id) {
            for &target_node_id in target_nodes {
                if let Some(target_node) = workflow.nodes.get(&target_node_id) {
                    message.node_id = target_node_id;
                    self.process_message(target_node, message);
                    // Recursively execute nodes connected to this target node
                    self.execute_connected_nodes(workflow, message, target_node_id);
                } else {
                    tracing::error!("Node {} not found in workflow", target_node_id);
                }
            }
        }
    }
} 