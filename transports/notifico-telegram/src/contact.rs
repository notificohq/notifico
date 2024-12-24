use notifico_core::error::EngineError;
use notifico_core::recipient::{RawContact, TypedContact};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramContact {
    pub chat_id: i64,
}

impl TryFrom<RawContact> for TelegramContact {
    type Error = EngineError;

    fn try_from(value: RawContact) -> Result<Self, Self::Error> {
        Ok(Self {
            chat_id: value.value.parse().map_err(|e| {
                EngineError::InvalidContactFormat(format!("chat_id must be an integer, got: {e:?}"))
            })?,
        })
    }
}

impl TypedContact for TelegramContact {
    const CONTACT_TYPE: &'static str = "telegram";
}
