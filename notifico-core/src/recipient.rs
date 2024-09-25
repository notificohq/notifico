use crate::error::EngineError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    pub id: Uuid,
    pub contacts: Vec<Contact>,
}

impl Recipient {
    pub fn get_primary_contact(&self, r#type: &str) -> Result<&Contact, EngineError> {
        for contact in &self.contacts {
            if contact.r#type() == r#type {
                return Ok(contact);
            }
        }
        Err(EngineError::ContactNotFound(r#type.to_string()))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contact(Value);

impl Contact {
    pub fn r#type(&self) -> &str {
        self.0["type"]
            .as_str()
            .expect("Contact type must be a string")
    }

    pub fn into_json(self) -> Value {
        self.0
    }
}

#[async_trait]
pub trait RecipientDirectory {
    async fn get_recipient(&self, id: Uuid) -> Option<Recipient>;
}
