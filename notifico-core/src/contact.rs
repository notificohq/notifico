use crate::error::EngineError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Contact {
    pub r#type: String,
    pub value: String,
}

impl FromStr for Contact {
    type Err = EngineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (r#type, value) = s.split_once(':').ok_or(EngineError::InvalidContactFormat(
            "Invalid URL format".to_string(),
        ))?;
        let (r#type, value) = (r#type.to_owned(), value.to_owned());
        Ok(Self { r#type, value })
    }
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
