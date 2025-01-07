use crate::entity::subscription;
use crate::entity::subscription::{Entity as Subscription, Model};
use async_trait::async_trait;
use migration::{Migrator, MigratorTrait, OnConflict};
use notifico_core::error::EngineError;
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityOrSelect, EntityTrait, PaginatorTrait,
    QueryFilter,
};
use serde::Serialize;
use tracing::error;
use uuid::Uuid;

pub struct SubscriptionDbController {
    db: DatabaseConnection,
}

impl SubscriptionDbController {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    pub async fn setup(&self) -> anyhow::Result<()> {
        Ok(Migrator::up(&self.db, None).await?)
    }

    pub async fn set_subscribed(
        &self,
        recipient_id: Uuid,
        event: &str,
        channel: &str,
        is_subscribed: bool,
    ) -> Result<(), EngineError> {
        // TODO: SECURITY: match channel against all supported channels, match event against all events in db
        let current_model = subscription::Entity::find()
            .filter(subscription::Column::RecipientId.eq(recipient_id))
            .filter(subscription::Column::Event.eq(event))
            .filter(subscription::Column::Channel.eq(channel))
            .one(&self.db)
            .await?;

        match current_model {
            Some(m) => {
                let mut am = subscription::ActiveModel::from(m);
                am.is_subscribed = Set(is_subscribed);
                am.update(&self.db).await?
            }
            None => {
                let am = subscription::ActiveModel {
                    id: Set(Uuid::now_v7()),
                    recipient_id: Set(recipient_id),
                    event: Set(event.to_string()),
                    channel: Set(channel.to_string()),
                    is_subscribed: Set(is_subscribed),
                };
                am.insert(&self.db).await?
            }
        };
        Ok(())
    }
    pub async fn is_subscribed(&self, recipient_id: Uuid, event: &str, channel: &str) -> bool {
        let result = Subscription
            .select()
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

#[derive(Clone, Serialize)]
pub struct SubscriptionItem {
    pub recipient_id: Uuid,
    pub event: String,
    pub channel: String,
    pub is_subscribed: bool,
}

impl From<subscription::Model> for SubscriptionItem {
    fn from(value: subscription::Model) -> Self {
        SubscriptionItem {
            recipient_id: value.recipient_id,
            event: value.event,
            channel: value.channel,
            is_subscribed: value.is_subscribed,
        }
    }
}

impl From<subscription::Model> for ItemWithId<SubscriptionItem> {
    fn from(value: subscription::Model) -> Self {
        ItemWithId {
            id: value.id,
            item: value.into(),
        }
    }
}

#[async_trait]
impl AdminCrudTable for SubscriptionDbController {
    type Item = SubscriptionItem;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError> {
        Ok(Subscription::find_by_id(id)
            .one(&self.db)
            .await?
            .map(|m| m.into()))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError> {
        let params = params.try_into()?;
        let total = Subscription::find()
            .apply_filter(&params)?
            .count(&self.db)
            .await?;

        let items = Subscription::find()
            .apply_params(&params)?
            .all(&self.db)
            .await?;

        Ok(PaginatedResult {
            items: items.into_iter().map(|m| m.into()).collect(),
            total,
        })
    }

    async fn create(&self, _entity: Self::Item) -> Result<ItemWithId<Self::Item>, EngineError> {
        todo!()
    }

    async fn update(
        &self,
        _id: Uuid,
        _entity: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, EngineError> {
        todo!()
    }

    async fn delete(&self, _id: Uuid) -> Result<(), EngineError> {
        todo!()
    }
}
