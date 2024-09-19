use notifico_core::error::EngineError;
use notifico_core::recipient::Contact;
use serde::Deserialize;
use teloxide::prelude::ChatId;
use teloxide::types::Recipient;

#[derive(Debug, Deserialize)]
pub struct TelegramContact {
    chat_id: ChatId,
}

impl TelegramContact {
    pub(crate) fn into_recipient(self) -> Recipient {
        Recipient::Id(self.chat_id)
    }
}

impl TryFrom<Contact> for TelegramContact {
    type Error = EngineError;

    fn try_from(value: Contact) -> Result<Self, Self::Error> {
        serde_json::from_value(value.into_json()).map_err(|_| EngineError::InvalidContactFormat)
    }
}
