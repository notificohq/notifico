use crate::error::EngineError;
use serde::{Deserialize, Serialize};
use serde_with::{DeserializeFromStr, SerializeDisplay};
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Clone, Debug, SerializeDisplay, DeserializeFromStr)]
pub struct RawContact {
    pub r#type: String,
    pub value: String,
}

impl FromStr for RawContact {
    type Err = EngineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (r#type, value) = s.split_once(':').ok_or(EngineError::InvalidContactFormat(
            "Invalid URL format".to_string(),
        ))?;
        let (r#type, value) = (r#type.to_owned(), value.to_owned());
        Ok(Self { r#type, value })
    }
}

impl Display for RawContact {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.r#type, self.value)
    }
}

pub trait TypedContact: TryFrom<RawContact, Error = EngineError> {
    const CONTACT_TYPE: &'static str;
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MobilePhoneContact {
    pub number: String,
}

impl TryFrom<RawContact> for MobilePhoneContact {
    type Error = EngineError;

    fn try_from(value: RawContact) -> Result<Self, Self::Error> {
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
