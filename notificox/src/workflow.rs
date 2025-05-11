use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;

pub type NodeId = u32;

#[derive(Serialize, Deserialize)]
pub struct SerializedNode {
    pub id: NodeId,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum NodeSlot {
    DefaultSlot(NodeId),
    NamedSlot { node: NodeId, slot: String },
}

impl NodeSlot {
    pub fn new(node: NodeId, slot: Option<String>) -> Self {
        if let Some(slot) = slot {
            Self::NamedSlot { node, slot }
        } else {
            Self::DefaultSlot(node)
        }
    }

    pub fn node(&self) -> NodeId {
        match self {
            Self::NamedSlot { node, .. } => *node,
            Self::DefaultSlot(node) => *node,
        }
    }

    pub fn slot(&self) -> Option<&str> {
        match self {
            Self::NamedSlot { slot, .. } => Some(slot),
            Self::DefaultSlot(_) => None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SerializedWorkflow {
    pub nodes: Vec<SerializedNode>,
    pub connections: Vec<[NodeSlot; 2]>,
}

pub struct ParsedWorkflow {
    pub nodes: HashMap<NodeId, SerializedNode>,
    pub connections: HashMap<NodeSlot, Vec<NodeSlot>>,
}

impl TryFrom<SerializedWorkflow> for ParsedWorkflow {
    type Error = String;

    fn try_from(workflow: SerializedWorkflow) -> Result<Self, Self::Error> {
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

        Ok(ParsedWorkflow { nodes, connections })
    }
}

impl ParsedWorkflow {
    pub fn find_trigger_nodes<'a>(
        &'a self,
        plugin_registry: &'a crate::plugin_registry::PluginRegistry,
    ) -> Vec<&'a SerializedNode> {
        self.nodes
            .values()
            .filter(|node| plugin_registry.is_trigger(&node.r#type))
            .collect()
    }
}
