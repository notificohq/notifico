use crate::crud_table::{
    AdminCrudError, AdminCrudTable, ItemWithId, ListQueryParams, ListableTrait, PaginatedResult,
};
use crate::entity;
use futures::FutureExt;
use metrics::{counter, gauge, Counter, Gauge};
use moka::future::Cache;
use moka::notification::ListenerFuture;
use sea_orm::ActiveValue::Unchanged;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, PaginatorTrait,
    QueryFilter, Set,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiKey {
    pub key: Uuid,
    pub description: String,
    pub project_id: Uuid,
    pub created_at: Option<chrono::NaiveDateTime>,
}

pub struct ApiKeyController {
    db: DatabaseConnection,
    authorization_cache: Cache<Uuid, Uuid>,
    authorization_cache_gauge: Gauge,
    authorization_cache_hit: Counter,
    authorization_cache_miss: Counter,
    authorization_invalid_key: Counter,
}

impl ApiKeyController {
    pub fn new(db: DatabaseConnection) -> Self {
        let authorization_cache_capacity = 100;
        gauge!("ingest_api_key_cache_capacity").set(authorization_cache_capacity as f64);

        let authorization_cache_gauge = gauge!("ingest_api_key_cache_total");
        let authorization_cache_gauge_for_fut = authorization_cache_gauge.clone();

        let authorization_cache = Cache::builder()
            .max_capacity(authorization_cache_capacity)
            .time_to_live(Duration::from_secs(1))
            .async_eviction_listener(move |_, _, _| -> ListenerFuture {
                authorization_cache_gauge_for_fut.decrement(1);
                async {}.boxed()
            })
            .build();

        Self {
            db,
            authorization_cache,
            authorization_cache_gauge,
            authorization_cache_hit: counter!("ingest_api_key_cache_hit"),
            authorization_cache_miss: counter!("ingest_api_key_cache_miss"),
            authorization_invalid_key: counter!("ingest_api_key_invalid"),
        }
    }
}

impl From<entity::api_key::Model> for ApiKey {
    fn from(value: entity::api_key::Model) -> Self {
        ApiKey {
            key: value.key,
            description: value.description,
            project_id: value.project_id,
            created_at: Some(value.created_at),
        }
    }
}

impl AdminCrudTable for ApiKeyController {
    type Item = ApiKey;

    async fn get_by_id(&self, id: Uuid) -> Result<Option<Self::Item>, AdminCrudError> {
        let query = entity::api_key::Entity::find_by_id(id)
            .one(&self.db)
            .await?;
        Ok(query.map(ApiKey::from))
    }

    async fn list(
        &self,
        params: ListQueryParams,
    ) -> Result<PaginatedResult<ItemWithId<Self::Item>>, AdminCrudError> {
        let params = params.try_into()?;
        let query = entity::api_key::Entity::find()
            .apply_params(&params)
            .unwrap()
            .all(&self.db)
            .await?;

        Ok(PaginatedResult {
            items: query
                .into_iter()
                .map(|m| ItemWithId {
                    id: m.id,
                    item: ApiKey::from(m),
                })
                .collect(),
            total: entity::api_key::Entity::find()
                .apply_filter(&params)?
                .count(&self.db)
                .await?,
        })
    }

    async fn create(&self, item: Self::Item) -> Result<ItemWithId<Self::Item>, AdminCrudError> {
        let id = Uuid::now_v7();
        let key = Uuid::new_v4();

        entity::api_key::ActiveModel {
            id: Set(id),
            key: Set(key),
            project_id: Set(item.project_id),
            description: Set(item.description.to_string()),
            created_at: NotSet,
        }
        .insert(&self.db)
        .await?;

        Ok(ItemWithId {
            id,
            item: ApiKey {
                key,
                description: item.description.to_string(),
                project_id: item.project_id,
                created_at: Some(chrono::Utc::now().naive_utc()),
            },
        })
    }

    async fn update(
        &self,
        id: Uuid,
        item: Self::Item,
    ) -> Result<ItemWithId<Self::Item>, AdminCrudError> {
        entity::api_key::ActiveModel {
            id: Unchanged(id),
            key: NotSet,
            project_id: NotSet,
            description: Set(item.description.to_string()),
            created_at: NotSet,
        }
        .update(&self.db)
        .await?;
        Ok(ItemWithId { id, item })
    }

    async fn delete(&self, id: Uuid) -> Result<(), AdminCrudError> {
        entity::api_key::Entity::delete_by_id(id)
            .exec(&self.db)
            .await?;
        Ok(())
    }
}

pub enum ApiKeyError {
    InvalidApiKey,
    InternalError,
}

impl ApiKeyController {
    pub async fn authorize_api_key(&self, key: &str) -> Result<Uuid, ApiKeyError> {
        let Ok(key_uuid) = Uuid::try_parse(key) else {
            self.authorization_invalid_key.increment(1);
            return Err(ApiKeyError::InvalidApiKey);
        };

        let cached_project_id = self.authorization_cache.get(&key_uuid).await;

        if let Some(project_id) = cached_project_id {
            // Cache Hit
            self.authorization_cache_hit.increment(1);
            Ok(project_id)
        } else {
            // Cache Miss
            self.authorization_cache_miss.increment(1);

            let Some(api_key_entry) = entity::api_key::Entity::find()
                .filter(entity::api_key::Column::Key.eq(key_uuid))
                .one(&self.db)
                .await
                .map_err(|_| ApiKeyError::InternalError)?
            else {
                self.authorization_invalid_key.increment(1);
                return Err(ApiKeyError::InvalidApiKey);
            };

            let project_id = api_key_entry.project_id;

            self.authorization_cache.insert(key_uuid, project_id).await;
            self.authorization_cache_gauge.increment(1);

            Ok(project_id)
        }
    }
}
