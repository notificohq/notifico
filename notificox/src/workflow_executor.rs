use crate::message::Message;
use crate::plugin::Outcome;
use crate::plugin_registry::PluginRegistry;
use crate::workflow::{NodeSlot, ParsedWorkflow, SerializedNode};
use std::collections::VecDeque;
use std::sync::Arc;
use tracing;

pub struct WorkflowExecutor {
    plugin_registry: Arc<PluginRegistry>,
}

impl WorkflowExecutor {
    pub fn new(plugin_registry: Arc<PluginRegistry>) -> Self {
        Self { plugin_registry }
    }

    pub fn find_trigger_node<'a>(
        &self,
        workflow: &'a ParsedWorkflow,
    ) -> Option<&'a SerializedNode> {
        workflow
            .nodes
            .values()
            .find(|node| self.plugin_registry.triggers.contains(&node.r#type))
    }

    pub async fn process_message(
        &self,
        node: &SerializedNode,
        message: Message,
        slot: Option<String>,
    ) -> Outcome {
        if let Some(plugin) = self.plugin_registry.nodes.get(&node.r#type) {
            tracing::info!(
                "Executing node {} of type {} with slot {:?}",
                node.id,
                node.r#type,
                slot
            );
            plugin.process_message(node, message, slot).await
        } else {
            tracing::warn!("No plugin found for node type: {}", node.r#type);
            Outcome::Error {
                error: format!("No plugin found for node type: {}", node.r#type),
            }
        }
    }

    pub async fn execute_workflow(&self, workflow: &ParsedWorkflow, message: Message) {
        // Find the trigger node
        let Some(trigger_node) = self.find_trigger_node(workflow) else {
            tracing::error!("No trigger node found in workflow");
            return;
        };

        // Execute the trigger node
        match self.process_message(trigger_node, message, None).await {
            Outcome::Return {
                message: new_message,
                slot,
            } => {
                // Execute all connected nodes iteratively
                self.execute_connected_nodes(
                    workflow,
                    new_message,
                    NodeSlot::new(trigger_node.id, slot),
                )
                .await;
            }
            Outcome::Error { error, .. } => {
                tracing::error!("Error executing trigger node: {}", error);
            }
        }
    }

    async fn execute_connected_nodes(
        &self,
        workflow: &ParsedWorkflow,
        message: Message,
        node_slot: NodeSlot,
    ) {
        // Create a queue to store nodes to process
        let mut queue = VecDeque::new();
        queue.push_back((message, node_slot));

        // Process nodes until the queue is empty
        while let Some((message, node_slot)) = queue.pop_front() {
            if let Some(target_slots) = workflow.connections.get(&node_slot) {
                for target_slot in target_slots {
                    if let Some(target_node) = workflow.nodes.get(&target_slot.node()) {
                        let mut new_message = message.clone();
                        new_message.node_id = target_slot.node();
                        match self
                            .process_message(
                                target_node,
                                new_message,
                                target_slot.slot().map(String::from),
                            )
                            .await
                        {
                            Outcome::Return {
                                message: new_message,
                                slot,
                            } => {
                                // Add the next node to the queue
                                queue.push_back((
                                    new_message,
                                    NodeSlot::new(target_slot.node(), slot),
                                ));
                            }
                            Outcome::Error { error, .. } => {
                                tracing::error!(
                                    "Error executing node {}: {}",
                                    target_slot.node(),
                                    error
                                );
                            }
                        }
                    } else {
                        tracing::error!("Node {} not found in workflow", target_slot.node());
                    }
                }
            }
        }
    }
}
