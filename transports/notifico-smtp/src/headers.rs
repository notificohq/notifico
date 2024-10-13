use lettre::message::header::{Header, HeaderName, HeaderValue};
use std::error::Error;

#[derive(Clone)]
pub struct ListUnsubscribe(String);

impl Header for ListUnsubscribe {
    fn name() -> HeaderName {
        HeaderName::new_from_ascii_str("List-Unsubscribe")
    }

    fn parse(s: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        Ok(Self(s.into()))
    }

    fn display(&self) -> HeaderValue {
        HeaderValue::new(Self::name(), self.0.clone())
    }
}

impl From<String> for ListUnsubscribe {
    #[inline]
    fn from(text: String) -> Self {
        Self(text)
    }
}
