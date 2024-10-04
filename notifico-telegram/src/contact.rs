use notifico_core::recipient::TypedContact;
use serde::Deserialize;
use teloxide::prelude::ChatId;
use teloxide::types::Recipient;

#[derive(Debug, Clone, Deserialize)]
pub struct TelegramContact {
    chat_id: ChatId,
}

impl TypedContact for TelegramContact {
    const CONTACT_TYPE: &'static str = "telegram";
}

impl TelegramContact {
    pub(crate) fn into_recipient(self) -> Recipient {
        Recipient::Id(self.chat_id)
    }
}
