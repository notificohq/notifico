use crate::error::EngineError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipient {
    #[serde(default = "String::new")]
    pub id: String,
    pub contacts: Vec<Contact>,
}

impl Recipient {
    pub fn get_primary_contact<T: TypedContact>(&self) -> Result<T, EngineError> {
        for contact in &self.contacts {
            if contact.r#type() == T::CONTACT_TYPE {
                return Ok(contact.clone().into_contact()?);
            }
        }
        Err(EngineError::ContactNotFound(T::CONTACT_TYPE.to_string()))
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

    pub fn into_contact<T>(self) -> Result<T, EngineError>
    where
        T: TypedContact + for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.0).map_err(|_| EngineError::InvalidContactFormat)
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
