use async_trait::async_trait;
use notifico_core::credentials::RawCredential;
use notifico_core::error::EngineError;
use notifico_core::pipeline::context::{Message, PipelineContext};
use notifico_core::recipient::RawContact;
use notifico_core::simpletransport::SimpleTransport;
use std::borrow::Cow;

pub struct NoopTransport {}

impl Default for NoopTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl NoopTransport {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl SimpleTransport for NoopTransport {
    async fn send_message(
        &self,
        _credential: RawCredential,
        _contact: RawContact,
        _message: Message,
        _context: &mut PipelineContext,
    ) -> Result<(), EngineError> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "noop"
    }

    fn has_contacts(&self) -> bool {
        true
    }

    fn supports_contact(&self, _type: &str) -> bool {
        true
    }

    fn supported_channels(&self) -> Vec<Cow<'static, str>> {
        vec!["noop".into()]
    }
}
