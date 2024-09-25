use actix::prelude::*;
use notifico_core::engine::{Engine, EventContext};
use notifico_core::pipeline::RecipientSelector;
use notifico_core::pipeline::{PipelineRunner, PipelineStorage};
use notifico_core::recipient::RecipientDirectory;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub struct EventHandler {
    pub(crate) pipeline_storage: Arc<dyn PipelineStorage>,
    pub(crate) engine: Engine,
    pub(crate) recipient_storage: Arc<dyn RecipientDirectory>,
}

impl Actor for EventHandler {
    type Context = actix::Context<Self>;
}

#[derive(actix::Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ProcessEvent {
    #[serde(default = "Uuid::nil")]
    pub(crate) project_id: Uuid,
    pub(crate) event: String,
    pub(crate) recipient: Option<RecipientSelector>,
    pub(crate) context: EventContext,
}

impl Handler<ProcessEvent> for EventHandler {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: ProcessEvent, _ctx: &mut Self::Context) -> Self::Result {
        let runner = PipelineRunner::new(
            self.pipeline_storage.clone(),
            self.engine.clone(),
            self.recipient_storage.clone(),
        );
        Box::pin(async move {
            runner
                .process_event(msg.project_id, &msg.event, msg.context, msg.recipient)
                .await
                .unwrap()
        })
    }
}
