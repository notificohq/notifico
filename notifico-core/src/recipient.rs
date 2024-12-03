use crate::error::EngineError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Recipient {
    #[serde(default = "Uuid::nil")]
    pub id: Uuid,
    pub contacts: Vec<Contact>,
}

impl Recipient {
    pub fn get_contacts(&self, channel: &str) -> Vec<Contact> {
        self.contacts
            .iter()
            .filter(|contact| contact.r#type() == channel)
            .cloned()
            .collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Contact(Value);

impl Contact {
    pub fn r#type(&self) -> &str {
        self.0["type"]
            .as_str()
            .expect("Contact type must be a string")
    }

    pub fn into_contact<T>(self) -> Result<T, EngineError>
    where
        T: TypedContact + for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.0).map_err(EngineError::InvalidContactFormat)
    }
}

pub trait TypedContact: for<'de> Deserialize<'de> {
    const CONTACT_TYPE: &'static str;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MobilePhoneContact {
    pub number: String,
}

impl TypedContact for MobilePhoneContact {
    const CONTACT_TYPE: &'static str = "mobile_phone";
}

impl MobilePhoneContact {
    pub fn msisdn(&self) -> &str {
        self.number.strip_prefix("+").unwrap_or(&self.number)
    }
}

#[async_trait]
pub trait RecipientDirectory {
    async fn get_recipient(&self, project: Uuid, id: &str) -> Option<Recipient>;
}
