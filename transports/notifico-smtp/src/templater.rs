use lettre::message::Mailbox;
use notifico_core::error::EngineError;
use notifico_core::templater::RenderedTemplate;
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct RenderedEmail {
    pub from: Mailbox,
    pub subject: String,
    pub body_html: String,
    pub body: String,
}

impl TryFrom<RenderedTemplate> for RenderedEmail {
    type Error = EngineError;

    fn try_from(value: RenderedTemplate) -> Result<Self, Self::Error> {
        Ok(Self {
            from: value
                .get("from")?
                .parse::<Mailbox>()
                .map_err(|e| EngineError::InvalidRenderedTemplateFormat(e.into()))?,
            subject: value.get("subject")?.to_string(),
            body_html: value.get("body_html")?.to_string(),
            body: value.get("body")?.to_string(),
        })
    }
}
