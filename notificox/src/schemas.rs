use serde::{Deserialize, Serialize};

pub type NodeId = u32;

#[derive(Serialize, Deserialize, Default, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    #[default]
    Action,
    Trigger,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SerializedNode {
    pub id: NodeId,
    pub r#type: String,
    pub kind: NodeKind,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Debug)]
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
