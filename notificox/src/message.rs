#[derive(Debug, Clone)]
pub struct Message {
    pub id: String,
    pub node_id: u32,
    pub data: serde_json::Value,
}

impl Message {
    pub fn new(node_id: u32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            node_id,
            data: serde_json::json!({}),
        }
    }
} 