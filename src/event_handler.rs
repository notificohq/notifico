use actix::prelude::*;
use notifico_core::engine::{Engine, EventContext};
use notifico_core::pipeline::RecipientSelector;
use notifico_core::pipeline::{PipelineRunner, PipelineStorage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

pub struct EventHandler {
    pub(crate) pipeline_storage: Arc<dyn PipelineStorage>,
    pub(crate) engine: Engine,
}

impl Actor for EventHandler {
    type Context = actix::Context<Self>;
}

#[derive(actix::Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct ProcessEventRequest {
    #[serde(default = "Uuid::now_v7")]
    pub(crate) id: Uuid,
    #[serde(default = "Uuid::nil")]
    pub(crate) project_id: Uuid,
    pub(crate) event: String,
    pub(crate) recipient: Option<RecipientSelector>,
    pub(crate) context: EventContext,
}

impl Handler<ProcessEventRequest> for EventHandler {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, msg: ProcessEventRequest, _ctx: &mut Self::Context) -> Self::Result {
        let runner = PipelineRunner::new(self.pipeline_storage.clone(), self.engine.clone());
        Box::pin(async move {
            runner
                .process_event(msg.project_id, &msg.event, msg.context, msg.recipient)
                .await
                .unwrap()
        })
    }
}
