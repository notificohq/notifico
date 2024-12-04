use crate::contact::{Contact, TypedContact};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Recipient {
    #[serde(default = "Uuid::nil")]
    pub id: Uuid,
    pub contacts: Vec<Contact>,
}

impl Recipient {
    pub fn get_contacts<T: TypedContact>(&self) -> Vec<T> {
        let mut contacts = vec![];
        for contact in self.contacts.iter() {
            if contact.r#type == T::CONTACT_TYPE {
                contacts.push(contact.clone().try_into().unwrap());
            }
        }
        contacts
    }
}
