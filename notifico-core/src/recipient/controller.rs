use crate::error::EngineError;
use crate::pipeline::event::RecipientSelector;
use crate::recipient::Recipient;
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use futures::StreamExt;
use tracing::warn;

#[async_trait]
pub trait RecipientController: Send + Sync {
    async fn get_recipients(
        &self,
        sel: Vec<RecipientSelector>,
    ) -> Result<BoxStream<Recipient>, EngineError>;
}

pub struct RecipientInlineController;

#[async_trait]
impl RecipientController for RecipientInlineController {
    async fn get_recipients(
        &self,
        sel: Vec<RecipientSelector>,
    ) -> Result<BoxStream<Recipient>, EngineError> {
        Ok(
            stream::iter(sel.into_iter().filter_map(|selector| match selector {
                RecipientSelector::Recipient(recipient) => Some(recipient),
                _ => {
                    warn!("Invalid recipient selector: {:?}", selector);
                    None
                }
            }))
            .boxed(),
        )
    }
}
