use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Item {
    pub data: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: Uuid,
    pub items: Vec<Item>,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            id: Uuid::now_v7(),
            items: Default::default(),
        }
    }
}
