use crate::message::Message;
use crate::plugin_registry::PluginRegistry;
use crate::schemas::{NodeId, NodeKind, NodeSlot, SerializedNode, SerializedWorkflow};
use flume::{Receiver, Sender};
use futures::future::select_all;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone)]
pub struct ParsedWorkflow {
    pub nodes: HashMap<NodeId, SerializedNode>,
    pub connections: HashMap<NodeSlot, Vec<NodeSlot>>,
}

impl ParsedWorkflow {
    pub fn new(workflow: SerializedWorkflow) -> Self {
        let nodes = workflow
            .nodes
            .into_iter()
            .map(|node| (node.id, node))
            .collect();

        let mut connections = HashMap::new();
        for [source, target] in workflow.connections {
            connections
                .entry(source)
                .or_insert_with(Vec::new)
                .push(target);
        }

        ParsedWorkflow { nodes, connections }
    }

    pub fn run_node(
        &self,
        inputs: HashMap<NodeSlot, Receiver<Message>>,
        outputs: HashMap<NodeSlot, Sender<Message>>,
        node_id: NodeId,
        registry: Arc<PluginRegistry>,
    ) {
        // Get the node from our nodes map
        let Some(node) = self.nodes.get(&node_id).cloned() else {
            return;
        };

        // Get the plugin for this node type
        let Some(plugin) = registry.get_plugin(&node.r#type).cloned() else {
            tracing::error!("No plugin found for node type: {}", node.r#type);
            return;
        };

        // Create a channel to signal when any input is ready
        let (tx, rx) = flume::bounded(1);

        // Spawn tasks to wait for each input
        for (slot, mut receiver) in inputs {
            let tx = tx.clone();
            tokio::spawn(async move {
                loop {
                    if let Ok(message) = receiver.recv_async().await {
                        let _ = tx.send_async((slot.clone(), message)).await;
                    } else {
                        break;
                    }
                }
            });
        }

        // Spawn the main processing task
        tokio::spawn(async move {
            // Main processing loop
            loop {
                // Wait for any input to be ready
                let (input_slot, message) = match rx.recv_async().await {
                    Ok(data) => data,
                    Err(_) => {
                        tracing::error!("Error receiving input signal for node {}", node_id);
                        break;
                    }
                };

                // Process the node using the plugin
                match plugin
                    .process_message(&node, message, input_slot.slot().map(String::from))
                    .await
                {
                    crate::plugin::Outcome::Return { message, slot } => {
                        // Find the output slot to send the signal to
                        let output_slot = match slot {
                            Some(slot_name) => NodeSlot::NamedSlot {
                                node: node_id,
                                slot: slot_name,
                            },
                            None => NodeSlot::DefaultSlot(node_id),
                        };

                        // Send message to the specified output
                        if let Some(sender) = outputs.get(&output_slot) {
                            let _ = sender.send(message);
                        } else {
                            tracing::error!(
                                "No output found for slot {:?} in node {}",
                                output_slot,
                                node_id
                            );
                        }
                    }
                    crate::plugin::Outcome::Error { error } => {
                        tracing::error!("Error processing node {}: {}", node_id, error);
                    }
                }
            }
        });
    }
}
