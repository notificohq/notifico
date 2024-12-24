mod context;
pub mod entity;
pub mod plugins;
mod step;

use crate::entity::subscription;
use entity::prelude::*;
use migration::{Migrator, MigratorTrait};
use notifico_core::error::EngineError;
use notifico_core::http::admin::{ListQueryParams, ListableTrait};
use sea_orm::sea_query::OnConflict;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait};
use sea_orm::{DatabaseConnection, EntityOrSelect, QueryFilter};
use tracing::error;
use uuid::Uuid;

pub struct SubscriptionController {
    db: DatabaseConnection,
}

impl SubscriptionController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
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

    pub async fn list_subscriptions(
        &self,
        params: ListQueryParams,
    ) -> Result<(Vec<subscription::Model>, u64), EngineError> {
        let mut query_count = Subscription::find();
        query_count = query_count.apply_params(&params).unwrap();
        let count = query_count.count(&self.db).await?;

        let mut query = Subscription::find();
        query = query.apply_params(&params).unwrap();

        let results = query.all(&self.db).await?;
        Ok((results, count))
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<subscription::Model>, EngineError> {
        let query = Subscription::find_by_id(id).one(&self.db).await?;
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
        Subscription::update(model).exec(&self.db).await?;
        Ok(())
    }
}
