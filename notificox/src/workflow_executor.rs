use std::sync::Arc;
use crate::workflow::{SerializedNode, ParsedWorkflow, NodeId, NodeSlot};
use crate::plugin_registry::PluginRegistry;
use crate::message::Message;
use crate::plugin::Outcome;
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

    pub fn process_message(&self, node: &SerializedNode, message: Message, slot: Option<String>) -> Outcome {
        if let Some(plugin) = self.plugin_registry.nodes.get(&node.r#type) {
            tracing::info!("Executing node {} of type {} with slot {:?}", node.id, node.r#type, slot);
            plugin.process_message(node, message, slot)
        } else {
            tracing::warn!("No plugin found for node type: {}", node.r#type);
            Outcome::Error { 
                message, 
                error: format!("No plugin found for node type: {}", node.r#type) 
            }
        }
    }

    pub fn execute_workflow(&self, workflow: &ParsedWorkflow, message: Message) {
        // Find the trigger node
        let Some(trigger_node) = self.find_trigger_node(workflow) else {
            tracing::error!("No trigger node found in workflow");
            return;
        };

        // Execute the trigger node
        match self.process_message(trigger_node, message, None) {
            Outcome::Return { message: new_message, slot } => {
                // Execute all connected nodes recursively
                self.execute_connected_nodes(workflow, new_message, NodeSlot::new(trigger_node.id, slot));
            }
            Outcome::Error { message, error } => {
                tracing::error!("Error executing trigger node: {}", error);
            }
        }
    }

    fn execute_connected_nodes(&self, workflow: &ParsedWorkflow, message: Message, node_slot: NodeSlot) {
        if let Some(target_slots) = workflow.connections.get(&node_slot) {
            for target_slot in target_slots {
                if let Some(target_node) = workflow.nodes.get(&target_slot.node()) {
                    let mut new_message = message.clone();
                    new_message.node_id = target_slot.node();
                    match self.process_message(target_node, new_message, target_slot.slot().map(String::from)) {
                        Outcome::Return { message: new_message, slot } => {
                            // Recursively execute nodes connected to this target node
                            self.execute_connected_nodes(workflow, new_message, NodeSlot::new(target_slot.node(), slot));
                        }
                        Outcome::Error { message, error } => {
                            tracing::error!("Error executing node {}: {}", target_slot.node(), error);
                        }
                    }
                } else {
                    tracing::error!("Node {} not found in workflow", target_slot.node());
                }
            }
        }
    }
}