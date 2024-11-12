use lettre::message::Mailbox;
use notifico_core::templater::RenderedTemplate;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Clone)]
pub struct RenderedEmail {
    pub from: Mailbox,
    pub subject: String,
    pub body_html: String,
    pub body_plaintext: String,
}

impl TryFrom<RenderedTemplate> for RenderedEmail {
    type Error = ();

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::from_iter(value.0)).map_err(|_| ())
    }
}
