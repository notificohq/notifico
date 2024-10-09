mod entity;
pub mod http;

use crate::entity::ncenter_notification;
use crate::entity::prelude::NcenterNotification;
use async_trait::async_trait;
use chrono::Utc;
use migration::{Migrator, MigratorTrait};
use notifico_core::engine::{EnginePlugin, PipelineContext, StepOutput};
use notifico_core::error::EngineError;
use notifico_core::pipeline::SerializedStep;
use sea_orm::ActiveValue::Set;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use uuid::Uuid;

pub struct NCenterPlugin {
    pub(crate) db: DatabaseConnection,
}

impl NCenterPlugin {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn setup(&self) -> Result<(), anyhow::Error> {
        Migrator::up(&self.db, None).await?;
        Ok(())
    }
}

#[async_trait]
impl EnginePlugin for NCenterPlugin {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let step: Step = step.clone().convert_step()?;

        match step {
            Step::Send => {
                let Some(recipient) = context.recipient.clone() else {
                    return Err(EngineError::RecipientNotSet);
                };

                for message in context.messages.iter() {
                    let model = ncenter_notification::ActiveModel {
                        id: Set(Uuid::now_v7()),
                        recipient_id: Set(recipient.id),
                        project_id: Set(context.project_id),
                        content: Set(serde_json::to_value(&message.0).unwrap()),
                        created_at: Set(Utc::now().naive_utc()),
                    };

                    NcenterNotification::insert(model)
                        .exec(&self.db)
                        .await
                        .unwrap();
                }
                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        vec!["ncenter.send".into()]
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "step")]
enum Step {
    #[serde(rename = "ncenter.send")]
    Send,
}
