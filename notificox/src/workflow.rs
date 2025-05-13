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
    }
}
