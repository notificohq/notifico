use async_trait::async_trait;
use notifico_core::recipient::{Recipient, RecipientDirectory};
use std::collections::HashMap;
use uuid::Uuid;

pub struct MemoryRecipientDirectory {
    directory: HashMap<Uuid, Recipient>,
}

impl MemoryRecipientDirectory {
    pub fn new(recipients: Vec<Recipient>) -> Self {
        MemoryRecipientDirectory {
            directory: HashMap::from_iter(recipients.into_iter().map(|r| (r.id, r))),
        }
    }
}

#[async_trait]
impl RecipientDirectory for MemoryRecipientDirectory {
    async fn get_recipient(&self, id: Uuid) -> Option<Recipient> {
        self.directory.get(&id).cloned()
    }
}
