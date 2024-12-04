use crate::error::EngineError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Contact {
    pub r#type: String,
    pub value: String,
}

pub trait TypedContact: TryFrom<Contact, Error = EngineError> {
    const CONTACT_TYPE: &'static str;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MobilePhoneContact {
    pub number: String,
}

impl TryFrom<Contact> for MobilePhoneContact {
    type Error = EngineError;

    fn try_from(value: Contact) -> Result<Self, Self::Error> {
        Ok(Self {
            number: value.value,
        })
    }
}

impl TypedContact for MobilePhoneContact {
    const CONTACT_TYPE: &'static str = "mobile_phone";
}

impl MobilePhoneContact {
    pub fn msisdn(&self) -> &str {
        self.number.strip_prefix("+").unwrap_or(&self.number)
    }
}
