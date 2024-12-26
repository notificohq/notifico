mod context;
pub mod entity;
pub mod plugins;
// mod recipient_controller;
mod step;

use crate::entity::subscription;
use async_trait::async_trait;
use entity::prelude::*;
use migration::{Migrator, MigratorTrait};
use notifico_core::error::EngineError;
use notifico_core::http::admin::{
    AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use sea_orm::sea_query::OnConflict;
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait};
use sea_orm::{DatabaseConnection, EntityOrSelect, QueryFilter};
use serde::Serialize;
use std::collections::HashMap;
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
}

#[derive(Clone, Serialize)]
pub struct SubscriptionItem {
    pub id: Uuid,
    pub project_id: Uuid,
    pub recipient_id: Uuid,
    pub event: String,
    pub channel: String,
    pub is_subscribed: bool,
}

impl From<subscription::Model> for SubscriptionItem {
    fn from(value: subscription::Model) -> Self {
        SubscriptionItem {
            id: value.id,
            project_id: value.project_id,
            recipient_id: value.recipient_id,
            event: value.event,
            channel: value.channel,
            is_subscribed: value.is_subscribed,
        }
    }
}

#[async_trait]
impl AdminCrudTable for SubscriptionController {
    type Item = SubscriptionItem;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, EngineError> {
        let query = Subscription::find_by_id(id)
            .one(&self.db)
            .await?
            .map(|m| m.into());
        Ok(query)
    }

    async fn list(
        &self,
        params: ListQueryParams,
        _extras: HashMap<String, String>,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, EngineError> {
        let mut query_count = Subscription::find();
        query_count = query_count.apply_params(&params).unwrap();
        let count = query_count.count(&self.db).await?;

        let mut query = Subscription::find();
        query = query.apply_params(&params).unwrap();

        let results = query.all(&self.db).await?;
        Ok(PaginatedResult {
            items: results
                .into_iter()
                .map(|model| ItemWithId {
                    id: model.id,
                    item: model.into(),
                })
                .collect(),
            total_count: count,
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
