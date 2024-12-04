use notifico_core::contact::{Contact, TypedContact};
use notifico_core::error::EngineError;
use serde::Deserialize;
use teloxide::prelude::ChatId;
use teloxide::types::Recipient;

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramContact {
    chat_id: ChatId,
}

impl TryFrom<Contact> for TelegramContact {
    type Error = EngineError;

    fn try_from(value: Contact) -> Result<Self, Self::Error> {
        Ok(Self {
            chat_id: ChatId(value.value.parse().map_err(|e| {
                EngineError::ContactTypeMismatch(format!("chat_id must be an integer, got: {e:?}"))
            })?),
        })
    }
}

impl TypedContact for TelegramContact {
    const CONTACT_TYPE: &'static str = "telegram";
}

impl TelegramContact {
    pub(crate) fn into_recipient(self) -> Recipient {
        Recipient::Id(self.chat_id)
    }
}
