mod context;
mod entity;
pub mod http;
mod step;

use crate::context::EMAIL_LIST_UNSUBSCRIBE;
use crate::entity::subscription;
use crate::step::STEPS;
use entity::prelude::*;
use jsonwebtoken::{EncodingKey, Header};
use migration::{Migrator, MigratorTrait};
use notifico_core::http::admin::{apply_list_params, ListQueryParams};
use notifico_core::http::auth::Claims;
use notifico_core::step::SerializedStep;
use notifico_core::{
    engine::PipelineContext,
    engine::{EnginePlugin, StepOutput},
    error::EngineError,
};
use sea_orm::prelude::async_trait::async_trait;
use sea_orm::sea_query::OnConflict;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait};
use sea_orm::{DatabaseConnection, EntityOrSelect, QueryFilter};
use serde_json::Value;
use std::borrow::Cow;
use std::time::{SystemTime, UNIX_EPOCH};
use step::Step;
use tracing::error;
use url::Url;
use uuid::Uuid;

pub struct SubscriptionManager {
    db: DatabaseConnection,
    secret_key: Vec<u8>,
    subscriber_url: Url,
}

impl SubscriptionManager {
    pub fn new(db: DatabaseConnection, secret_key: Vec<u8>, subscriber_url: Url) -> Self {
        Self {
            db,
            secret_key,
            subscriber_url,
        }
    }

    pub async fn setup(&self) -> anyhow::Result<()> {
        Ok(Migrator::up(&self.db, None).await?)
    }

    pub async fn set_subscribed(
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
            recipient_id: Set(recipient_id),
            event: Set(event.to_string()),
            channel: Set(channel.to_string()),
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

impl SubscriptionManager {
    pub async fn list_subscriptions(
        &self,
        params: ListQueryParams,
    ) -> Result<(Vec<subscription::Model>, u64), EngineError> {
        let mut query_count = Subscription::find();
        query_count = apply_list_params(query_count, params.clone()).unwrap();
        let count = query_count.count(&self.db).await.unwrap();

        let mut query = Subscription::find();
        query = apply_list_params(query, params).unwrap();

        let results = query.all(&self.db).await.unwrap();
        Ok((results, count))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<subscription::Model>, EngineError> {
        let query = Subscription::find_by_id(id).one(&self.db).await.unwrap();
        Ok(query)
    }

    pub async fn update_subscription(
        &self,
        id: Uuid,
        is_subscribed: bool,
    ) -> Result<(), EngineError> {
        let model = subscription::ActiveModel {
            id: Set(id),
            is_subscribed: Set(is_subscribed),
            ..Default::default()
        };
        Subscription::update(model).exec(&self.db).await.unwrap();
        Ok(())
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
    key: Vec<u8>,
    subscriber_url: Url,
    project_id: Uuid,
    event: &str,
    recipient_id: Uuid,
) -> Url {
    let claims = Claims {
        proj: project_id,
        sub: recipient_id,
        scopes: [String::from("list_unsubscribe")].into_iter().collect(),
        exp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 60 * 60 * 24 * 30,
    };

    let token =
        jsonwebtoken::encode(&Header::default(), &claims, &EncodingKey::from_secret(&key)).unwrap();

    //TODO: Optimize URL creation to avoid format machinery
    subscriber_url
        .join(&format!(
            "api/recipient/v1/list_unsubscribe?token={}&event={}",
            token, event
        ))
        .unwrap()
}
