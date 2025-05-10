use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryFrom;

pub type NodeId = u32;

#[derive(Serialize, Deserialize)]
pub struct SerializedNode {
    pub id: NodeId,
    pub r#type: String,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedWorkflow {
    pub nodes: Vec<SerializedNode>,
    pub connections: Vec<[NodeId; 2]>,
}

pub struct ParsedWorkflow {
    pub nodes: HashMap<NodeId, SerializedNode>,
    pub connections: HashMap<NodeId, Vec<NodeId>>,
}

impl TryFrom<SerializedWorkflow> for ParsedWorkflow {
    type Error = String;

    fn try_from(workflow: SerializedWorkflow) -> Result<Self, Self::Error> {
        let nodes = workflow.nodes
            .into_iter()
            .map(|node| (node.id, node))
            .collect();

        let mut connections = HashMap::new();
        for [source, target] in workflow.connections {
            connections.entry(source)
                .or_insert_with(Vec::new)
                .push(target);
        }

        Ok(ParsedWorkflow { 
            nodes, 
            connections 
        })
    }
}