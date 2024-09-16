use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct Recipient {
    pub id: Uuid,
    pub contacts: Vec<Contact>,
}

impl Recipient {
    pub fn get_primary_contact(&self, r#type: &str) -> Option<&Contact> {
        for contact in &self.contacts {
            if contact.r#type() == r#type {
                return Some(&contact);
            }
        }
        None
    }
}

#[derive(Clone, Debug, Deserialize)]
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
