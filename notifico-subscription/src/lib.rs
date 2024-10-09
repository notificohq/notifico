mod context;
mod entity;
pub mod http;
mod step;

use crate::context::EMAIL_LIST_UNSUBSCRIBE;
use crate::entity::subscription;
use crate::step::STEPS;
use entity::prelude::*;
use hmac::Hmac;
use jwt::SignWithKey;
use notifico_core::{
    engine::PipelineContext,
    engine::{EnginePlugin, StepOutput},
    error::EngineError,
    pipeline::SerializedStep,
};
use sea_orm::prelude::async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait};
use sea_orm::{DatabaseConnection, EntityOrSelect, QueryFilter};
use serde_json::Value;
use sha2::Sha256;
use std::borrow::Cow;
use std::collections::BTreeMap;
use step::Step;
use tracing::error;
use url::Url;
use uuid::Uuid;

pub struct SubscriptionManager {
    db: DatabaseConnection,
    secret_key: Hmac<Sha256>,
    subscriber_url: Url,
}

impl SubscriptionManager {
    pub fn new(db: DatabaseConnection, secret_key: Hmac<Sha256>, subscriber_url: Url) -> Self {
        Self {
            db,
            secret_key,
            subscriber_url,
        }
    }

    pub async fn unsubscribe(
        &self,
        project_id: Uuid,
        recipient_id: Uuid,
        event: &str,
        channel: &str,
        is_subscribed: bool,
    ) {
        let model = subscription::ActiveModel {
            id: Set(Uuid::now_v7()),
            project_id: Set(project_id),
            event: Set(event.to_string()),
            channel: Set(channel.to_string()),
            recipient_id: Set(recipient_id),
            is_subscribed: Set(is_subscribed),
        };

        subscription::Entity::insert(model)
            .on_conflict(
                OnConflict::columns([
                    subscription::Column::ProjectId,
                    subscription::Column::RecipientId,
                    subscription::Column::Event,
                    subscription::Column::Channel,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec(&self.db)
            .await
            .unwrap();
    }
    pub async fn is_subscribed(
        &self,
        project_id: Uuid,
        recipient_id: Uuid,
        event: &str,
        channel: &str,
    ) -> bool {
        let result = Subscription
            .select()
            .filter(subscription::Column::ProjectId.eq(project_id))
            .filter(subscription::Column::RecipientId.eq(recipient_id))
            .filter(subscription::Column::Event.eq(event))
            .filter(subscription::Column::Channel.eq(channel))
            .one(&self.db)
            .await;
        match result {
            Ok(Some(subscription)) => subscription.is_subscribed,
            Ok(None) => true,
            Err(e) => {
                error!("Error checking subscription: {}", e);
                false
            }
        }
    }
}

#[async_trait]
impl EnginePlugin for SubscriptionManager {
    async fn execute_step(
        &self,
        context: &mut PipelineContext,
        step: &SerializedStep,
    ) -> Result<StepOutput, EngineError> {
        let Some(recipient) = &context.recipient else {
            return Err(EngineError::RecipientNotSet);
        };

        let step: Step = step.clone().convert_step()?;

        match step {
            Step::Check { channel } => {
                if self
                    .is_subscribed(
                        context.project_id,
                        recipient.id,
                        &context.trigger_event,
                        &channel,
                    )
                    .await
                {
                    Ok(StepOutput::Continue)
                } else {
                    Ok(StepOutput::Interrupt)
                }
            }
            Step::ListUnsubscribe { .. } => {
                let Some(recipient) = context.recipient.clone() else {
                    return Err(EngineError::RecipientNotSet);
                };

                context.plugin_contexts.insert(
                    EMAIL_LIST_UNSUBSCRIBE.into(),
                    Value::String(format!(
                        "<{}>",
                        create_self_unsubscribe_url(
                            self.secret_key.clone(),
                            self.subscriber_url.clone(),
                            context.project_id,
                            &context.trigger_event,
                            recipient.id,
                            "email"
                        )
                    )),
                );
                Ok(StepOutput::Continue)
            }
        }
    }

    fn steps(&self) -> Vec<Cow<'static, str>> {
        STEPS.iter().map(|&s| s.into()).collect()
    }
}

// Implements one-click List-Unsubscribe style URL generation
pub fn create_self_unsubscribe_url(
    key: Hmac<Sha256>,
    subscriber_url: Url,
    project_id: Uuid,
    event: &str,
    recipient_id: Uuid,
    channel: &str,
) -> Url {
    let mut claims = BTreeMap::new();
    claims.insert("sub", recipient_id.to_string());
    claims.insert("proj", project_id.to_string());

    let token = claims.sign_with_key(&key).unwrap();
    let url = format!(
        "{}/unsubscribe?token={}&event={}&channel={}",
        subscriber_url, token, event, channel
    );
    Url::parse(&url).unwrap()
}
