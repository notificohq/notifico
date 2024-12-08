use crate::contact::{Contact, TypedContact};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Recipient {
    #[serde(default = "Uuid::nil")]
    /// This ID will be used for List-Unsubscribe and other features.
    /// It is recommended to store it in an external system and use the same ID for the same Recipient.
    pub id: Uuid,
    #[schema(value_type = Vec<String>, examples("telegram:123456789", "mobile_phone:+123456789", "email:Anyone <anyone@example.com>"))]
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
